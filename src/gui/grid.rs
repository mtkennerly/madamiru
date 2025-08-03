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
    prelude::Change,
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
    PauseChanged { category: player::Category, paused: bool },
    MuteChanged,
    RelativePositionChanged { category: player::Category, position: f64 },
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
        let players = if settings.sources.is_empty() {
            vec![]
        } else {
            vec![Player::default()]
        };

        Self {
            sources: settings.sources.clone(),
            players,
            content_fit: settings.content_fit,
            orientation: settings.orientation,
            orientation_limit: settings.orientation_limit,
        }
    }

    pub fn new_with_players(settings: &Settings, players: usize) -> Self {
        Self {
            sources: settings.sources.clone(),
            players: std::iter::repeat_with(Player::default).take(players).collect(),
            content_fit: settings.content_fit,
            orientation: settings.orientation,
            orientation_limit: settings.orientation_limit,
        }
    }

    fn playback(&self, playback: &Playback) -> Playback {
        playback
            .with_paused_maybe(self.all_paused())
            .with_muted_maybe(self.all_muted())
    }

    pub fn is_idle(&self) -> bool {
        self.players.is_empty()
    }

    pub fn tick(&mut self, elapsed: Duration, collection: &mut media::Collection, playback: &Playback) {
        let playback = self.playback(playback);

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
                    player::Update::PauseChanged(_) => {}
                    player::Update::MuteChanged => {}
                    player::Update::RelativePositionChanged(_) => {}
                    player::Update::EndOfStream => {
                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[index];

                        match media {
                            Some(media) => {
                                if player.swap_media(&media, &playback).is_err() {
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
        let playback = self.playback(playback);

        for player in &mut self.players {
            player.reload_audio(&playback);
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

    #[must_use]
    pub fn set_settings(&mut self, settings: Settings) -> Change {
        if self.players.is_empty() && settings.sources.iter().any(|x| !x.is_empty()) {
            self.players.push(Player::default());
        }

        if self.settings() == settings {
            return Change::Same;
        }

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

        Change::Different
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

    pub fn player_ids(&self) -> Vec<player::Id> {
        self.players
            .iter()
            .enumerate()
            .map(|(i, _player)| player::Id(i))
            .collect()
    }

    pub fn refresh(&mut self, collection: &mut media::Collection, playback: &Playback, context: media::RefreshContext) {
        let playback = self.playback(playback);
        let mut active: HashSet<_> = self.active_media().into_iter().cloned().collect();
        let force = match context {
            media::RefreshContext::Launch => false,
            media::RefreshContext::Edit => false,
            media::RefreshContext::Playlist => false,
            media::RefreshContext::Automatic => false,
            media::RefreshContext::Manual => true,
        };

        for player in self.players.iter_mut() {
            if player.is_error() && !force {
                continue;
            }

            let old_media = player.media();
            let refresh = force
                || old_media
                    .map(|old_media| collection.is_outdated(old_media, &self.sources))
                    .unwrap_or(true)
                || player.is_error();

            if refresh {
                if let Some(old_media) = old_media {
                    active.remove(old_media);
                }

                match collection.one_new(&self.sources, active.iter().collect()) {
                    Some(new_media) => {
                        if player.swap_media(&new_media, &playback).is_err() {
                            collection.mark_error(&new_media);
                        }
                        active.insert(new_media);
                    }
                    None => {
                        player.go_idle();
                    }
                }
            }
        }
    }

    pub fn add_player(&mut self, collection: &mut media::Collection, playback: &Playback) -> Result<(), Error> {
        let playback = self.playback(playback);

        let Some(media) = collection.one_new(&self.sources, self.active_media()) else {
            return Err(Error::NoMediaAvailable);
        };

        match Player::new(&media, &playback) {
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

    pub fn player(&self, player_id: player::Id) -> Option<&Player> {
        self.players.get(player_id.0)
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
        let playback = self.playback(playback);

        match event {
            Event::Player { player_id, event } => {
                let active_media: HashSet<_> = self.active_media().into_iter().cloned().collect();
                let player = self.players.get_mut(player_id.0)?;
                let category = player.category();

                match player.update(event, &playback) {
                    Some(update) => match update {
                        player::Update::MuteChanged => Some(Update::MuteChanged),
                        player::Update::PauseChanged(paused) => {
                            self.synchronize_players(
                                Some(player_id),
                                Some(category),
                                player::Event::SetPause(paused),
                                &playback,
                            );
                            Some(Update::PauseChanged { category, paused })
                        }
                        player::Update::RelativePositionChanged(position) => {
                            self.synchronize_players(
                                Some(player_id),
                                Some(category),
                                player::Event::SeekRelative(position),
                                &playback,
                            );
                            Some(Update::RelativePositionChanged { category, position })
                        }
                        player::Update::EndOfStream => {
                            let media = collection.one_new(&self.sources, active_media.iter().collect());

                            match media {
                                Some(media) => {
                                    if player.swap_media(&media, &playback).is_err() {
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
                                    if player.swap_media(&media, &playback).is_err() {
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
                        player::Update::Close => {
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
        let playback = self.playback(playback).with_synchronized(false);

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
                &playback,
            );
        }
    }

    pub fn synchronize_players(
        &mut self,
        originator: Option<player::Id>,
        category: Option<player::Category>,
        event: player::Event,
        playback: &Playback,
    ) {
        if !playback.synchronized {
            return;
        }
        for (i, player) in self.players.iter_mut().enumerate() {
            if Some(player::Id(i)) == originator || category.is_some_and(|x| x != player.category()) {
                continue;
            }
            let _ = player.update(event.clone(), playback);
        }
    }

    pub fn view(
        &self,
        grid_id: Id,
        selected: bool,
        selected_player: Option<player::Id>,
        obscured: bool,
        dragging_file: bool,
    ) -> Element {
        let obscured = obscured || dragging_file;

        let mut row = Row::new().spacing(5);
        let mut column = Column::new().spacing(5);
        let mut count = 0;
        let limit = match self.orientation_limit {
            OrientationLimit::Automatic => self.calculate_row_limit(),
            OrientationLimit::Fixed(limit) => limit.get(),
        };

        for (i, player) in self.players.iter().enumerate() {
            let player_id = player::Id(i);
            let selected_player = selected_player == Some(player_id);
            let new = Container::new(player.view(
                grid_id,
                player_id,
                selected || selected_player,
                obscured,
                self.content_fit,
            ))
            .padding(5)
            .class(style::Container::Player {
                selected: selected_player,
            });

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
                .class(style::Container::Player { selected: false });
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
