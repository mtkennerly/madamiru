use std::sync::Mutex;

use fluent::{bundle::FluentBundle, FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use regex::Regex;
use std::sync::LazyLock;
use unic_langid::LanguageIdentifier;

use crate::prelude::Error;

const VERSION: &str = "version";

/// Display language.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Language {
    /// English
    #[default]
    #[serde(rename = "en-US")]
    English,

    /// French
    #[serde(rename = "fr-FR")]
    French,

    /// German
    #[serde(rename = "de-DE")]
    German,

    /// Polish
    #[serde(rename = "pl-PL")]
    Polish,

    /// Brazilian Portuguese
    #[serde(rename = "pt-BR")]
    PortugueseBrazilian,
}

impl Language {
    pub const ALL: &'static [Self] = &[
        Self::German,
        Self::English,
        Self::French,
        Self::Polish,
        Self::PortugueseBrazilian,
    ];

    pub fn id(&self) -> LanguageIdentifier {
        let id = match self {
            Self::English => "en-US",
            Self::French => "fr-FR",
            Self::German => "de-DE",
            Self::Polish => "pl-PL",
            Self::PortugueseBrazilian => "pt-BR",
        };
        id.parse().unwrap()
    }

    fn name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Polish => "Polski",
            Self::PortugueseBrazilian => "Português brasileiro",
        }
    }

    fn completion(&self) -> u8 {
        match self {
            Self::English => 100,
            Self::French => 2,
            Self::German => 2,
            Self::Polish => 87,
            Self::PortugueseBrazilian => 1,
        }
    }
}

impl ToString for Language {
    fn to_string(&self) -> String {
        match self {
            Self::English => self.name().to_string(),
            _ => format!("{} ({}%)", self.name(), self.completion()),
        }
    }
}

static LANGUAGE: Mutex<Language> = Mutex::new(Language::English);

static BUNDLE: LazyLock<Mutex<FluentBundle<FluentResource, IntlLangMemoizer>>> = LazyLock::new(|| {
    let ftl = include_str!("../lang/en-US.ftl").to_owned();
    let res = FluentResource::try_new(ftl).expect("Failed to parse Fluent file content.");

    let mut bundle = FluentBundle::new_concurrent(vec![Language::English.id()]);
    bundle.set_use_isolating(false);

    bundle
        .add_resource(res)
        .expect("Failed to add Fluent resources to the bundle.");

    Mutex::new(bundle)
});

fn set_language(language: Language) {
    let mut bundle = BUNDLE.lock().unwrap();

    let ftl = match language {
        Language::English => include_str!("../lang/en-US.ftl"),
        Language::French => include_str!("../lang/fr-FR.ftl"),
        Language::German => include_str!("../lang/de-DE.ftl"),
        Language::Polish => include_str!("../lang/pl-PL.ftl"),
        Language::PortugueseBrazilian => include_str!("../lang/pt-BR.ftl"),
    }
    .to_owned();

    let res = FluentResource::try_new(ftl).expect("Failed to parse Fluent file content.");
    bundle.locales = vec![language.id()];

    bundle.add_resource_overriding(res);

    let mut last_language = LANGUAGE.lock().unwrap();
    *last_language = language;
}

static RE_EXTRA_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([^\r\n ]) {2,}").unwrap());
static RE_EXTRA_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([^\r\n ])[\r\n]([^\r\n ])").unwrap());
static RE_EXTRA_PARAGRAPHS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([^\r\n ])[\r\n]{2,}([^\r\n ])").unwrap());

fn translate(id: &str) -> String {
    translate_args(id, &FluentArgs::new())
}

fn translate_args(id: &str, args: &FluentArgs) -> String {
    let bundle = match BUNDLE.lock() {
        Ok(x) => x,
        Err(_) => return "fluent-cannot-lock".to_string(),
    };

    let parts: Vec<&str> = id.splitn(2, '.').collect();
    let (name, attr) = if parts.len() < 2 {
        (id, None)
    } else {
        (parts[0], Some(parts[1]))
    };

    let message = match bundle.get_message(name) {
        Some(x) => x,
        None => return format!("fluent-no-message={name}"),
    };

    let pattern = match attr {
        None => match message.value() {
            Some(x) => x,
            None => return format!("fluent-no-message-value={id}"),
        },
        Some(attr) => match message.get_attribute(attr) {
            Some(x) => x.value(),
            None => return format!("fluent-no-attr={id}"),
        },
    };
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, Some(args), &mut errors);

    RE_EXTRA_PARAGRAPHS
        .replace_all(
            &RE_EXTRA_LINES.replace_all(&RE_EXTRA_SPACES.replace_all(&value, "${1} "), "${1} ${2}"),
            "${1}\n\n${2}",
        )
        .to_string()
}

pub fn set(language: Language) {
    set_language(Language::English);
    if language != Language::English {
        set_language(language);
    }
}

pub fn app_name() -> String {
    "Madamiru".to_string()
}

pub fn window_title() -> String {
    let name = app_name();
    format!("{} v{}", name, *crate::prelude::VERSION)
}

pub fn field(text: &str) -> String {
    let language = LANGUAGE.lock().unwrap();
    match *language {
        Language::French => format!("{text} :"),
        _ => format!("{text}:"),
    }
}

pub fn handle_error(error: &Error) -> String {
    let error = match error {
        Error::ConfigInvalid { why } => format!("{}\n\n{why}", tell::config_is_invalid()),
        Error::NoMediaFound => tell::no_media_found_in_sources(),
        Error::PlaylistInvalid { why } => format!("{}\n\n{why}", tell::playlist_is_invalid()),
        Error::UnableToOpenPath(path) => format!("{}\n\n{}", tell::unable_to_open_path(), path.render()),
        Error::UnableToOpenUrl(url) => format!("{}\n\n{}", tell::unable_to_open_url(), url),
        Error::UnableToSavePlaylist { why } => format!("{}\n\n{why}", tell::unable_to_save_playlist()),
    };

    format!("{} {}", field(&thing::error()), error)
}

macro_rules! join {
    ($a:expr, $b:expr) => {
        format!("{} {}", $a, $b)
    };
}

pub(crate) use join;

pub mod thing {
    use super::*;

    pub fn application() -> String {
        translate("thing-application")
    }

    pub fn audio() -> String {
        translate("thing-audio")
    }

    pub fn content_fit() -> String {
        translate("thing-content-fit")
    }

    pub fn error() -> String {
        translate("thing-error")
    }

    pub fn glob() -> String {
        translate("thing-glob")
    }

    pub fn image() -> String {
        translate("thing-image")
    }

    pub fn items_per_line() -> String {
        translate("thing-items-per-line")
    }

    pub fn language() -> String {
        translate("thing-language")
    }

    pub fn layout() -> String {
        translate("thing-layout")
    }

    pub fn orientation() -> String {
        translate("thing-orientation")
    }

    pub fn path() -> String {
        translate("thing-path")
    }

    pub fn playlist() -> String {
        translate("thing-playlist")
    }

    pub fn settings() -> String {
        translate("thing-settings")
    }

    pub fn sources() -> String {
        translate("thing-sources")
    }

    pub fn theme() -> String {
        translate("thing-theme")
    }

    pub mod key {
        use super::*;

        pub fn shift() -> String {
            translate("thing-key-shift")
        }
    }
}

pub mod action {
    use super::*;

    pub fn add_player() -> String {
        translate("action-add-player")
    }

    pub fn cancel() -> String {
        translate("action-cancel")
    }

    pub fn check_for_updates() -> String {
        translate("action-check-for-updates")
    }

    pub fn close() -> String {
        translate("action-close")
    }

    pub fn confirm() -> String {
        translate("action-confirm")
    }

    pub fn confirm_when_discarding_unsaved_playlist() -> String {
        translate("action-confirm-when-discarding-unsaved-playlist")
    }

    pub fn crop() -> String {
        translate("action-crop")
    }

    pub fn exit_app() -> String {
        translate("action-exit-app")
    }

    pub fn jump_position() -> String {
        translate("action-jump-position")
    }

    pub fn mute() -> String {
        translate("action-mute")
    }

    pub fn open_folder() -> String {
        translate("action-open-folder")
    }

    pub fn open_file() -> String {
        translate("action-open-file")
    }

    pub fn open_playlist() -> String {
        translate("action-open-playlist")
    }

    pub fn pause() -> String {
        translate("action-pause")
    }

    pub fn pause_when_window_loses_focus() -> String {
        translate("action-pause-when-window-loses-focus")
    }

    pub fn play() -> String {
        translate("action-play")
    }

    pub fn play_for_this_many_seconds() -> String {
        translate("action-play-for-this-many-seconds")
    }

    pub fn save_playlist() -> String {
        translate("action-save-playlist")
    }

    pub fn save_playlist_as_new_file() -> String {
        translate("action-save-playlist-as-new-file")
    }

    pub fn scale() -> String {
        translate("action-scale")
    }

    pub fn scale_down() -> String {
        translate("action-scale-down")
    }

    pub fn select_folder() -> String {
        translate("action-select-folder")
    }

    pub fn select_file() -> String {
        translate("action-select-file")
    }

    pub fn shuffle() -> String {
        translate("action-shuffle")
    }

    pub fn split_horizontally() -> String {
        translate("action-split-horizontally")
    }

    pub fn split_vertically() -> String {
        translate("action-split-vertically")
    }

    pub fn start_new_playlist() -> String {
        translate("action-start-new-playlist")
    }

    pub fn stretch() -> String {
        translate("action-stretch")
    }

    pub fn unmute() -> String {
        translate("action-unmute")
    }

    pub fn view_releases() -> String {
        translate("action-view-releases")
    }
}

pub mod state {
    use super::*;

    pub fn dark() -> String {
        translate("state-dark")
    }

    pub fn horizontal() -> String {
        translate("state-horizontal")
    }

    pub fn light() -> String {
        translate("state-light")
    }

    pub fn vertical() -> String {
        translate("state-vertical")
    }
}

pub mod tell {
    use super::*;

    pub fn config_is_invalid() -> String {
        translate("tell-config-is-invalid")
    }

    pub fn player_will_loop() -> String {
        translate("tell-player-will-loop")
    }

    pub fn player_will_shuffle() -> String {
        translate("tell-player-will-shuffle")
    }

    pub fn playlist_has_unsaved_changes() -> String {
        translate("tell-playlist-has-unsaved-changes")
    }

    pub fn playlist_is_invalid() -> String {
        translate("tell-playlist-is-invalid")
    }

    pub fn new_version_available(version: &str) -> String {
        let mut args = FluentArgs::new();
        args.set(VERSION, version);
        translate_args("tell-new-version-available", &args)
    }

    pub fn no_media_found_in_sources() -> String {
        translate("tell-no-media-found-in-sources")
    }

    #[allow(unused)]
    pub fn unable_to_determine_media_duration() -> String {
        translate("tell-unable-to-determine-media-duration")
    }

    pub fn unable_to_open_path() -> String {
        translate("tell-unable-to-open-path")
    }

    pub fn unable_to_open_url() -> String {
        translate("tell-unable-to-open-url")
    }

    pub fn unable_to_save_playlist() -> String {
        translate("tell-unable-to-save-playlist")
    }
}

pub mod ask {
    use super::*;

    pub fn discard_changes() -> String {
        translate("ask-discard-changes")
    }

    pub fn load_new_playlist_anyway() -> String {
        translate("ask-load-new-playlist-anyway")
    }

    pub fn view_release_notes() -> String {
        translate("ask-view-release-notes")
    }
}
