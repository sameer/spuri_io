use actix_web::http::header::{CacheControl, CacheDirective};

const TEN_YEARS_IN_SECONDS: u32 = ONE_WEEK_IN_SECONDS * 52 * 10;

pub fn cache_forever() -> CacheControl {
    CacheControl(vec![
        CacheDirective::Public,
        CacheDirective::MaxAge(TEN_YEARS_IN_SECONDS),
    ])
}

const ONE_WEEK_IN_SECONDS: u32 = ONE_DAY_IN_SECONDS * 7;

pub fn cache_for_one_week() -> CacheControl {
    CacheControl(vec![
        CacheDirective::Public,
        CacheDirective::MaxAge(ONE_WEEK_IN_SECONDS),
    ])
}

const ONE_DAY_IN_SECONDS: u32 = 60 * 60 * 24;

pub fn cache_for_one_day() -> CacheControl {
    CacheControl(vec![
        CacheDirective::Public,
        CacheDirective::MaxAge(ONE_DAY_IN_SECONDS),
    ])
}
