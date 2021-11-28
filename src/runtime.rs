use evlog::Logger;
use once_cell::sync::OnceCell;

pub static LOGGER: OnceCell<Logger> = OnceCell::new();

pub fn set_logger(l: Logger) {
    LOGGER.set(l).ok().unwrap();
}

pub fn get_logger<'a>() -> &'a Logger {
    LOGGER.get().unwrap()
}
