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
    resource::config::Playback,
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

#[derive(Default)]
pub struct Grid {
    sources: Vec<media::Source>,
    players: Vec<Player>,
}

impl Grid {
    pub fn new(sources: &[media::Source]) -> Self {
        Self {
            sources: sources.to_vec(),
            players: vec![Player::Idle],
        }
    }

    pub fn is_idle(&self) -> bool {
        self.players.is_empty() || (self.players.len() == 1 && matches!(self.players[0], Player::Idle))
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

    pub fn sources(&self) -> &[media::Source] {
        &self.sources
    }

    pub fn set_sources(&mut self, sources: Vec<media::Source>) {
        self.sources = sources;
    }

    fn active_media(&self) -> HashSet<&Media> {
        self.players.iter().filter_map(|x| x.media()).collect()
    }

    pub fn refresh(&mut self, collection: &mut media::Collection, playback: &Playback) {
        let total = if self.is_idle() {
            playback.max_initial_media.get()
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
            self.players.push(Player::Idle);
        }
    }

    fn refresh_outdated(&mut self, collection: &mut media::Collection, playback: &Playback) {
        let mut remove = vec![];
        let mut active: HashSet<_> = self.active_media().into_iter().cloned().collect();

        for (index, player) in self.players.iter_mut().enumerate() {
            if let Some(old_media) = player.media() {
                if collection.is_outdated(old_media, &self.sources) {
                    active.remove(old_media);
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
        }

        for id in remove.into_iter().rev() {
            self.players.remove(id.0);
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
            Event::Player { player_id, event } => match self.players[player_id.0].update(event, playback) {
                Some(update) => match update {
                    player::Update::MuteChanged { .. } => Some(Update::MuteChanged),
                    player::Update::PauseChanged { .. } => Some(Update::PauseChanged),
                    player::Update::EndOfStream { .. } => {
                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[player_id.0];

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
                        let failed = self.players[player_id.0].is_error();

                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[player_id.0];

                        match media {
                            Some(media) => {
                                if player.swap_media(&media, playback).is_err() {
                                    collection.mark_error(&media);
                                }
                            }
                            None => {
                                if failed {
                                    self.remove(player_id);
                                    if self.players.is_empty() {
                                        self.players.push(Player::Idle);
                                    }
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
                        if self.players.is_empty() {
                            self.players.push(Player::Idle);
                        }
                        Some(Update::PlayerClosed)
                    }
                },
                None => None,
            },
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
        let limit = self.calculate_row_limit();

        for (i, player) in self.players.iter().enumerate() {
            row = row.push(
                Container::new(player.view(grid_id, player::Id(i), obscured))
                    .padding(5)
                    .class(style::Container::Player),
            );
            count += 1;

            if count == limit {
                count = 0;
                column = column.push(row);
                row = Row::new().spacing(5);
            }
        }

        column = column.push(row);

        Stack::new()
            .push(column)
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
                    .tooltip_below(if all_muted {
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
                    .tooltip_below(if all_paused {
                        lang::action::play()
                    } else {
                        lang::action::pause()
                    })
            }))
            .push_maybe(show_player_controls.then(|| {
                button::mini_icon(Icon::Refresh)
                    .on_press(Message::Pane {
                        event: PaneEvent::Refresh { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip_below(lang::action::shuffle_media())
            }))
            .push_maybe(show_player_controls.then(|| {
                button::mini_icon(Icon::TimerRefresh)
                    .on_press(Message::Pane {
                        event: PaneEvent::SeekRandom { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip_below(lang::action::jump_position())
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
                    .tooltip_below(lang::action::split_vertically()),
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
                    .tooltip_below(lang::action::split_horizontally()),
            )
            .push(
                button::mini_icon(Icon::Add)
                    .on_press(Message::Pane {
                        event: PaneEvent::AddPlayer { grid_id },
                    })
                    .enabled(!self.is_idle())
                    .obscured(obscured)
                    .tooltip_below(lang::action::add_player()),
            )
            .push(
                button::mini_icon(Icon::PlaylistAdd)
                    .on_press(Message::Pane {
                        event: PaneEvent::ShowSources { grid_id },
                    })
                    .obscured(obscured)
                    .tooltip_below(lang::action::configure_media_sources()),
            )
            .push(
                button::mini_icon(Icon::Close)
                    .on_press(Message::Pane {
                        event: PaneEvent::Close { grid_id },
                    })
                    .enabled(has_siblings)
                    .obscured(obscured)
                    .tooltip_below(lang::action::close()),
            )
            .into()
    }
}
