pub mod home;
pub mod login;
pub mod register;
pub mod settings;

pub use home::home_page;
pub use login::login_page;
pub use register::register_page;
pub use settings::{get_settings_account, settings_page};
