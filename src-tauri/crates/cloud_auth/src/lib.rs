//! The login flow that `cloud-sync` leaves to the application, for the services that need one.
//!
//! A sign in is the authorization code flow with PKCE. [`begin`] returns the page the user has to
//! visit, and [`Pending::wait`] turns the answer into tokens. How the browser finds its way back
//! depends on the platform, see `Redirect`. Desktop apps open a listener on a free local port and
//! the service sends the browser there. Mobile apps register a URL scheme with the operating
//! system, which hands the link to the app, and the app passes it on with [`deliver_redirect`].
//!
//! The refresh token is kept in a file by [`TokenStore`]. A [`Session`] hands out short lived
//! access tokens from it and is what `cloud-sync` calls through its `TokenProvider` trait.
//!
//! The OAuth client of each service is registered by whoever builds the app, and its id is read
//! from the environment at compile time, see [`configured`].

mod app;
mod error;
mod flow;
mod link;
mod session;
mod store;
mod token;

pub use app::{check, configured, display_name};
pub use error::{Error, Result};
pub use flow::{Pending, begin};
pub use link::deliver_redirect;
pub use session::Session;
pub use store::{Saved, TokenStore};
pub use token::SignedIn;
