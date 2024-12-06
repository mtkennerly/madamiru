use std::{collections::HashSet, time::Duration};

use crate::{
    gui::{
        player::{self, Player},
        style,
        widget::{Column, Container, Element, Row},
    },
    media::{self, Media},
    resource::config::Playback,
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

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
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
            Event::Player { id, event } => match self.players[id.0].update(event, playback) {
                Some(update) => match update {
                    player::Update::MuteChanged { .. } => Some(Update::MuteChanged),
                    player::Update::PauseChanged { .. } => Some(Update::PauseChanged),
                    player::Update::EndOfStream { .. } => {
                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[id.0];

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
                        let failed = self.players[id.0].is_error();

                        let media = collection.one_new(&self.sources, self.active_media());
                        let player = &mut self.players[id.0];

                        match media {
                            Some(media) => {
                                if player.swap_media(&media, playback).is_err() {
                                    collection.mark_error(&media);
                                }
                            }
                            None => {
                                if failed {
                                    self.remove(id);
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

    pub fn update_all_players(
        &mut self,
        event: player::Event,
        collection: &mut media::Collection,
        playback: &Playback,
    ) {
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
                collection,
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
