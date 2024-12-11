use std::{collections::HashSet, time::Duration};

use iced::{
    alignment, padding,
    widget::{pane_grid, vertical_rule},
    Length,
};

use crate::{
    gui::{
        button,
        common::{Message, PaneEvent},
        icon::Icon,
        player::{self, Player},
        style,
        widget::{Column, Container, Element, Row, Stack},
    },
    lang,
    media::{self, Media},
    resource::{
        config::Playback,
        playlist::{ContentFit, Orientation, OrientationLimit},
    },
};

pub type Id = pane_grid::Pane;

#[derive(Debug)]
pub enum Error {
    NoMediaAvailable,
}

#[derive(Debug, Clone)]
pub enum Event {
    Player {
        player_id: player::Id,
        event: player::Event,
    },
}

#[derive(Debug, Clone)]
pub enum Update {
    PauseChanged,
    MuteChanged,
    PlayerClosed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub sources: Vec<media::Source>,
    pub content_fit: ContentFit,
    pub orientation: Orientation,
    pub orientation_limit: OrientationLimit,
}

impl Settings {
    pub fn with_source(mut self, source: media::Source) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_sources(mut self, sources: Vec<media::Source>) -> Self {
        self.sources.extend(sources);
        self
    }
}

#[derive(Default)]
pub struct Grid {
    sources: Vec<media::Source>,
    players: Vec<Player>,
    content_fit: ContentFit,
    orientation: Orientation,
    orientation_limit: OrientationLimit,
}

impl Grid {
    pub fn new(settings: &Settings) -> Self {
        Self {
            sources: settings.sources.clone(),
            players: vec![],
            content_fit: settings.content_fit,
            orientation: settings.orientation,
            orientation_limit: settings.orientation_limit,
        }
    }

    pub fn new_with_players(settings: &Settings, players: usize) -> Self {
        Self {
            sources: settings.sources.clone(),
            players: std::iter::repeat_with(|| Player::Idle).take(players).collect(),
            content_fit: settings.content_fit,
            orientation: settings.orientation,
            orientation_limit: settings.orientation_limit,
        }
    }

    pub fn is_idle(&self) -> bool {
        self.players.is_empty()
    }

    pub fn tick(&mut self, elapsed: Duration, collection: &mut media::Collection, playback: &Playback) {
        let updates: Vec<_> = self
            .players
            .iter_mut()
            .enumerate()
            .rev()
            .map(|(index, player)| (index, player.tick(elapsed)))
            .collect();

        for (index, update) in updates {
            if let Some(update) = update {
                match update {
                    player::Update::PauseChanged => {}
                    player::Update::MuteChanged => {}
                    player::Update::EndOfStream => {
                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[index];

                        match media {
                            Some(media) => {
                                if player.swap_media(&media, playback).is_err() {
                                    collection.mark_error(&media);
                                }
                            }
                            None => {
                                player.restart();
                            }
                        }
                    }
                    player::Update::Refresh => {}
                    player::Update::Close => {}
                }
            }
        }
    }

    #[cfg(feature = "audio")]
    pub fn reload_audio(&mut self, playback: &Playback) {
        for player in &mut self.players {
            player.reload_audio(playback);
        }
    }

    pub fn remove(&mut self, id: player::Id) {
        self.players.remove(id.0);
    }

    pub fn all_paused(&self) -> Option<bool> {
        let mut relevant = false;
        for player in &self.players {
            match player.is_paused() {
                Some(true) => {
                    relevant = true;
                }
                Some(false) => {
                    return Some(false);
                }
                None => {}
            }
        }

        relevant.then_some(true)
    }

    pub fn all_muted(&self) -> Option<bool> {
        let mut relevant = false;
        for player in &self.players {
            match player.is_muted() {
                Some(true) => {
                    relevant = true;
                }
                Some(false) => {
                    return Some(false);
                }
                None => {}
            }
        }

        relevant.then_some(true)
    }

    pub fn can_jump(&self) -> bool {
        self.players.iter().any(|player| player.can_jump())
    }

    pub fn settings(&self) -> Settings {
        Settings {
            sources: self.sources.clone(),
            content_fit: self.content_fit,
            orientation: self.orientation,
            orientation_limit: self.orientation_limit,
        }
    }

    pub fn set_settings(&mut self, settings: Settings) {
        let Settings {
            sources,
            content_fit,
            orientation,
            orientation_limit,
        } = settings;

        self.sources = sources;
        self.content_fit = content_fit;
        self.orientation = orientation;
        self.orientation_limit = orientation_limit;
    }

    pub fn sources(&self) -> &[media::Source] {
        &self.sources
    }

    fn active_media(&self) -> HashSet<&Media> {
        self.players.iter().filter_map(|x| x.media()).collect()
    }

    pub fn total_players(&self) -> usize {
        self.players.len()
    }

    pub fn refresh(&mut self, collection: &mut media::Collection, playback: &Playback) {
        let total = if self.is_idle() {
            media::MAX_INITIAL
        } else {
            self.players.len()
        };

        if let Some(media) = collection.new_first(&self.sources, total, self.active_media()) {
            self.players.clear();

            for item in media {
                match Player::new(&item, playback) {
                    Ok(player) => {
                        self.players.push(player);
                    }
                    Err(player) => {
                        collection.mark_error(&item);
                        self.players.push(player);
                    }
                }
            }
        } else {
            self.players.clear();
        }
    }

    fn refresh_outdated(&mut self, collection: &mut media::Collection, playback: &Playback) {
        let mut remove = vec![];
        let mut active: HashSet<_> = self.active_media().into_iter().cloned().collect();

        for (index, player) in self.players.iter_mut().enumerate() {
            if player.is_error() {
                continue;
            }

            let old_media = player.media();
            let outdated = old_media
                .map(|old_media| collection.is_outdated(old_media, &self.sources))
                .unwrap_or(true);

            if outdated {
                if let Some(old_media) = old_media {
                    active.remove(old_media);
                }

                match collection.one_new(&self.sources, active.iter().collect()) {
                    Some(new_media) => {
                        if player.swap_media(&new_media, playback).is_err() {
                            collection.mark_error(&new_media);
                        }
                        active.insert(new_media);
                    }
                    None => {
                        remove.push(player::Id(index));
                    }
                }
            }
        }

        for id in remove.into_iter().rev() {
            self.remove(id);
        }
    }

    pub fn refresh_on_media_collection_changed(
        &mut self,
        context: media::RefreshContext,
        collection: &mut media::Collection,
        playback: &Playback,
    ) {
        match context {
            media::RefreshContext::Launch => {
                self.refresh(collection, playback);
            }
            media::RefreshContext::Edit => {
                if self.is_idle() {
                    self.refresh(collection, playback);
                } else {
                    self.refresh_outdated(collection, playback);
                }
            }
            media::RefreshContext::Playlist => {
                self.refresh(collection, playback);
            }
            media::RefreshContext::Automatic => {
                self.refresh_outdated(collection, playback);
            }
        }
    }

    pub fn add_player(&mut self, collection: &mut media::Collection, playback: &Playback) -> Result<(), Error> {
        let Some(media) = collection.one_new(&self.sources, self.active_media()) else {
            return Err(Error::NoMediaAvailable);
        };

        if self.is_idle() {
            self.players.clear();
        }

        match Player::new(&media, playback) {
            Ok(player) => {
                self.players.push(player);
            }
            Err(player) => {
                collection.mark_error(&media);
                self.players.push(player);
            }
        }

        Ok(())
    }

    fn calculate_row_limit(&self) -> usize {
        let mut limit = 1;
        loop {
            if self.players.len() > limit * limit {
                limit += 1;
            } else {
                break;
            }
        }
        limit
    }

    #[must_use]
    pub fn update(&mut self, event: Event, collection: &mut media::Collection, playback: &Playback) -> Option<Update> {
        match event {
            Event::Player { player_id, event } => {
                let active_media: HashSet<_> = self.active_media().into_iter().cloned().collect();
                let player = self.players.get_mut(player_id.0)?;

                match player.update(event, playback) {
                    Some(update) => match update {
                        player::Update::MuteChanged { .. } => Some(Update::MuteChanged),
                        player::Update::PauseChanged { .. } => Some(Update::PauseChanged),
                        player::Update::EndOfStream { .. } => {
                            let media = collection.one_new(&self.sources, active_media.iter().collect());

                            match media {
                                Some(media) => {
                                    if player.swap_media(&media, playback).is_err() {
                                        collection.mark_error(&media);
                                    }
                                }
                                None => {
                                    player.restart();
                                }
                            }

                            None
                        }
                        player::Update::Refresh => {
                            let failed = player.is_error();

                            let media = collection.one_new(&self.sources, active_media.iter().collect());

                            match media {
                                Some(media) => {
                                    if player.swap_media(&media, playback).is_err() {
                                        collection.mark_error(&media);
                                    }
                                }
                                None => {
                                    if failed {
                                        self.remove(player_id);
                                        return Some(Update::PlayerClosed);
                                    } else {
                                        player.restart();
                                    }
                                }
                            }

                            None
                        }
                        player::Update::Close { .. } => {
                            self.remove(player_id);
                            Some(Update::PlayerClosed)
                        }
                    },
                    None => None,
                }
            }
        }
    }

    pub fn update_all_players(
        &mut self,
        event: player::Event,
        collection: &mut media::Collection,
        playback: &Playback,
    ) {
        let player_ids: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .map(|(id, _)| player::Id(id))
            .rev()
            .collect();
        for player_id in player_ids {
            let _ = self.update(
                Event::Player {
                    player_id,
                    event: event.clone(),
                },
                collection,
                playback,
            );
        }
    }

    pub fn view(&self, grid_id: Id, obscured: bool, dragging_file: bool) -> Element {
        let obscured = obscured || dragging_file;

        let mut row = Row::new().spacing(5);
        let mut column = Column::new().spacing(5);
        let mut count = 0;
        let limit = match self.orientation_limit {
            OrientationLimit::Automatic => self.calculate_row_limit(),
            OrientationLimit::Fixed(limit) => limit.get(),
        };

        for (i, player) in self.players.iter().enumerate() {
            let new = Container::new(player.view(grid_id, player::Id(i), obscured, self.content_fit))
                .padding(5)
                .class(style::Container::Player);

            match self.orientation {
                Orientation::Horizontal => {
                    row = row.push(new);
                }
                Orientation::Vertical => {
                    column = column.push(new);
                }
            }
            count += 1;

            if count == limit {
                count = 0;
                match self.orientation {
                    Orientation::Horizontal => {
                        column = column.push(row);
                        row = Row::new().spacing(5);
                    }
                    Orientation::Vertical => {
                        row = row.push(column);
                        column = Column::new().spacing(5);
                    }
                }
            }
        }

        let mut body = match self.orientation {
            Orientation::Horizontal => Container::new(column.push(row)),
            Orientation::Vertical => Container::new(row.push(column)),
        };

        if self.players.is_empty() {
            body = Container::new("")
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(5)
                .class(style::Container::Player);
        }

        Stack::new()
            .push(body)
            .push_maybe(
                dragging_file.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::FileDrag),
                ),
            )
            .push_maybe(
                dragging_file.then_some(
                    Container::new(
                        button::max_icon(Icon::PlaylistAdd).on_press(Message::FileDragDropGridSelected(grid_id)),
                    )
                    .center(Length::Fill),
                ),
            )
            .into()
    }

    pub fn controls(&self, grid_id: Id, obscured: bool, has_siblings: bool) -> Element<'_> {
        let show_player_controls = has_siblings && !self.is_idle();

        Row::new()
            .align_y(alignment::Vertical::Center)
            .push_maybe(self.all_muted().filter(|_| show_player_controls).map(|all_muted| {
                button::mini_icon(if all_muted { Icon::Mute } else { Icon::VolumeHigh })
                    .on_press(Message::Pane {
                        event: PaneEvent::SetMute {
                            grid_id,
                            muted: !all_muted,
                        },
                    })
                    .obscured(obscured)
                    .tooltip(if all_muted {
                        lang::action::unmute()
                    } else {
                        lang::action::mute()
                    })
            }))
            .push_maybe(self.all_paused().filter(|_| show_player_controls).map(|all_paused| {
                button::mini_icon(if all_paused { Icon::Play } else { Icon::Pause })
                    .on_press(Message::Pane {
                        event: PaneEvent::SetPause {
                            grid_id,
                            paused: !all_paused,
                        },
                    })
                    .obscured(obscured)
                    .tooltip(if all_paused {
                        lang::action::play()
                    } else {
                        lang::action::pause()
                    })
            }))
            .push_maybe((show_player_controls && self.can_jump()).then(|| {
                button::mini_icon(Icon::TimerRefresh)
                    .on_press(Message::Pane {
                        event: PaneEvent::SeekRandom { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip(lang::action::jump_position())
            }))
            .push_maybe(show_player_controls.then(|| {
                button::mini_icon(Icon::Refresh)
                    .on_press(Message::Pane {
                        event: PaneEvent::Refresh { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip(lang::action::shuffle())
            }))
            .push_maybe(show_player_controls.then(|| {
                Container::new(vertical_rule(2))
                    .height(10)
                    .padding(padding::left(5).right(5))
            }))
            .push(
                button::mini_icon(Icon::SplitVertical)
                    .on_press(Message::Pane {
                        event: PaneEvent::Split {
                            grid_id,
                            axis: pane_grid::Axis::Horizontal,
                        },
                    })
                    .obscured(obscured)
                    .tooltip(lang::action::split_vertically()),
            )
            .push(
                button::mini_icon(Icon::SplitHorizontal)
                    .on_press(Message::Pane {
                        event: PaneEvent::Split {
                            grid_id,
                            axis: pane_grid::Axis::Vertical,
                        },
                    })
                    .obscured(obscured)
                    .tooltip(lang::action::split_horizontally()),
            )
            .push(
                button::mini_icon(Icon::Add)
                    .on_press(Message::Pane {
                        event: PaneEvent::AddPlayer { grid_id },
                    })
                    .enabled(!self.sources.is_empty())
                    .obscured(obscured)
                    .tooltip(lang::action::add_player()),
            )
            .push(
                button::mini_icon(Icon::Settings)
                    .on_press(Message::Pane {
                        event: PaneEvent::ShowSettings { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip(lang::thing::settings()),
            )
            .push(
                button::mini_icon(Icon::Close)
                    .on_press(Message::Pane {
                        event: PaneEvent::Close { grid_id },
                    })
                    .enabled(has_siblings)
                    .obscured(obscured)
                    .tooltip(lang::action::close()),
            )
            .into()
    }
}
