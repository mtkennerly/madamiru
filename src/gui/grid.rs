use std::collections::HashSet;

use crate::{
    gui::{
        player::{self, Player},
        style,
        widget::{Column, Container, Element, Row},
    },
    path::StrictPath,
    resource::config::Playback,
    scan,
};

#[derive(Debug)]
pub enum Error {
    NoMediaAvailable,
}

#[derive(Debug, Clone)]
pub enum Event {
    Player { id: player::Id, event: player::Event },
}

#[derive(Debug, Clone)]
pub enum Update {
    PauseChanged,
    MuteChanged,
    PlayerClosed,
}

pub struct Grid {
    sources: Vec<StrictPath>,
    players: Vec<Player>,
}

impl Grid {
    pub fn new(sources: &[StrictPath], playback: &Playback) -> Self {
        match scan::find_videos(sources, playback.max) {
            Some(videos) => Self {
                sources: sources.to_vec(),
                players: videos.into_iter().map(|x| Player::video(&x, playback)).collect(),
            },
            None => Grid {
                sources: sources.to_vec(),
                players: vec![Player::Idle],
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn is_idle(&self) -> bool {
        self.players.is_empty() || (self.players.len() == 1 && matches!(self.players[0], Player::Idle))
    }

    pub fn remove(&mut self, id: player::Id) {
        self.players.remove(id.0);
    }

    pub fn all_paused(&self) -> bool {
        if self.is_empty() {
            false
        } else {
            self.players.iter().all(|x| x.is_paused().is_some_and(|x| x))
        }
    }

    pub fn all_muted(&self) -> bool {
        if self.is_empty() {
            false
        } else {
            self.players.iter().all(|x| x.is_muted().is_some_and(|x| x))
        }
    }

    pub fn sources(&self) -> &[StrictPath] {
        &self.sources
    }

    pub fn set_sources(&mut self, sources: Vec<StrictPath>, playback: &Playback) {
        self.sources = sources;
        self.refresh(playback);
    }

    fn active_sources(&self) -> HashSet<&StrictPath> {
        self.players.iter().filter_map(|x| x.source()).collect()
    }

    pub fn refresh(&mut self, playback: &Playback) {
        let total = if self.is_idle() {
            playback.max
        } else {
            self.players.len()
        };

        if let Some(videos) = scan::find_new_videos_first(&self.sources, usize::MAX, total, self.active_sources()) {
            self.players.clear();

            for video in videos {
                let player = Player::video(&video, playback);
                self.players.push(player);
            }
        } else {
            self.players.clear();
            self.players.push(Player::Idle);
        }
    }

    pub fn add_player(&mut self, playback: &Playback) -> Result<(), Error> {
        let Some(video) = scan::find_new_video(&self.sources, usize::MAX, self.active_sources()) else {
            return Err(Error::NoMediaAvailable);
        };

        if self.is_idle() {
            self.players.clear();
        }

        let player = Player::video(&video, playback);
        self.players.push(player);

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
    pub fn update(&mut self, event: Event, playback: &Playback) -> Option<Update> {
        match event {
            Event::Player { id, event } => match self.players[id.0].update(event) {
                Some(update) => match update {
                    player::Update::MuteChanged => Some(Update::MuteChanged),
                    player::Update::PauseChanged => Some(Update::PauseChanged),
                    player::Update::EndOfStream => {
                        let video = scan::find_new_video(&self.sources, usize::MAX, self.active_sources());
                        let player = &mut self.players[id.0];

                        match video {
                            Some(video) => {
                                let playback = playback.with_muted_maybe(player.is_muted());
                                player.swap_video(&video, &playback);
                            }
                            None => {
                                player.restart();
                            }
                        }

                        None
                    }
                    player::Update::Close => {
                        self.remove(id);
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

    pub fn update_all_players(&mut self, event: player::Event, playback: &Playback) {
        let ids: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .map(|(id, _)| player::Id(id))
            .rev()
            .collect();
        for id in ids {
            let _ = self.update(
                Event::Player {
                    id,
                    event: event.clone(),
                },
                playback,
            );
        }
    }

    pub fn view(&self, obscured: bool) -> Element {
        let mut row = Row::new().spacing(5);
        let mut column = Column::new().spacing(5);
        let mut count = 0;
        let limit = self.calculate_row_limit();

        for (i, player) in self.players.iter().enumerate() {
            row = row.push(
                Container::new(player.view(player::Id(i), obscured))
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

        column.into()
    }
}
