pub(crate) mod server_helper;

pub(crate) mod client_helper;

#[derive(PartialEq)]
pub(crate) enum Recovery {
    Retry,
    None,
}
