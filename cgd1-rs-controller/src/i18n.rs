//! Internationalization support using the Fluent ecosystem.
//!
//! Embeds `.ftl` translation files at compile time via `rust-embed`
//! and loads the system locale at runtime via `i18n-embed`.

use std::sync::OnceLock;

use i18n_embed::DesktopLanguageRequester;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::fluent::fluent_language_loader;
use rust_embed::RustEmbed;

/// Embeds the `i18n/` directory containing `.ftl` files into the binary.
#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

/// The global fluent language loader.
///
/// Initialized once at first access with the system's preferred language,
/// falling back to English (`en`) as the default/fallback language.
static LOADER: OnceLock<FluentLanguageLoader> = OnceLock::new();

/// Get the global language loader, initializing it on first access.
pub fn loader() -> &'static FluentLanguageLoader {
    LOADER.get_or_init(|| {
        let loader = fluent_language_loader!();

        let requested = DesktopLanguageRequester::requested_languages();
        let languages = i18n_embed::select(&loader, &Localizations, &requested);

        if let Err(error) = languages {
            tracing::warn!(error = %error, "failed to load requested languages, falling back to default");
        }

        loader
    })
}

/// Convenience macro wrapping `i18n_embed_fl::fl!` with the static loader.
///
/// Usage:
/// ```
/// use cgd1_rs_controller::fl;
///
/// let label = fl!("status-disconnected");
/// ```
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::loader(), $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::i18n::loader(), $message_id, $($args), *)
    }};
}

/// Get a translated message by ID without compile-time checks.
///
/// Use this for dynamic message IDs (e.g. weekday names looked up by key).
/// For static messages, prefer the `fl!` macro.
pub fn get(message_id: &str) -> String {
    loader().get(message_id)
}

/// Get a translated message with a single string argument.
#[allow(dead_code)]
pub fn get_str(message_id: &str, arg_name: &str, value: &str) -> String {
    let mut args = fluent_bundle::FluentArgs::with_capacity(1);
    args.set(arg_name, value);
    loader().get_args_fluent(message_id, Some(&args))
}

/// Get a translated message with a single integer argument.
pub fn get_int(message_id: &str, arg_name: &str, value: i64) -> String {
    let mut args = fluent_bundle::FluentArgs::with_capacity(1);
    args.set(arg_name, value);
    loader().get_args_fluent(message_id, Some(&args))
}

/// Initialize the i18n system.
///
/// This forces the lazy loader to be initialized with the system locale.
/// Call this early in `main()` to ensure translations are ready before
/// any UI code runs.
pub fn init() {
    let _ = loader();
}
