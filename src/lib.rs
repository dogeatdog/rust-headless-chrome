//! A high-level API to control headless Chrome or Chromium over the DevTools Protocol. It is the
//! Rust equivalent of [Puppeteer](https://github.com/GoogleChrome/puppeteer), a Node library
//! maintained by the Chrome DevTools team.
//!
//! It is not 100% feature compatible with Puppeteer, but there's enough here to satisfy most
//! browser testing / web crawling use cases, and there are several 'advanced' features such as:
//!
//! - [network request interception](https://docs.rs/headless_chrome/latest/headless_chrome/browser/tab/struct.Tab.html#method.enable_request_interception)
//! - [JavaScript coverage monitoring](https://docs.rs/headless_chrome/latest/headless_chrome/browser/tab/struct.Tab.html#method.take_precise_js_coverage)
//! - [taking screenshots of elements or the entire page](https://docs.rs/headless_chrome/latest/headless_chrome/browser/tab/struct.Tab.html#method.capture_screenshot)
//! - [saving pages to PDF](https://docs.rs/headless_chrome/latest/headless_chrome/browser/tab/struct.Tab.html#method.print_to_pdf)
//! - ['headful' browsing](https://docs.rs/headless_chrome/latest/headless_chrome/struct.LaunchOptionsBuilder.html#method.headless)
//! - automatic downloading of 'known good' Chromium binaries for Linux / Mac / Windows
//! - [extension pre-loading](https://docs.rs/headless_chrome/latest/headless_chrome/struct.LaunchOptionsBuilder.html#method.extensions)
//!
//! # Quick Start
//!
//! ```no_run
//! use headless_chrome::{Browser, protocol::page::ScreenshotFormat};
//! use headless_chrome::protocol::cdp::Page;
//!
//! fn browse_wikipedia() -> Result<(), failure::Error> {
//!     let browser = Browser::default()?;
//!
//!     let tab = browser.new_tab()?;
//!
//!     /// Navigate to wikipedia
//!     tab.navigate_to("https://www.wikipedia.org")?;
//!
//!     /// Wait for network/javascript/dom to make the search-box available
//!     /// and click it.
//!     tab.wait_for_element("input#searchInput")?.click()?;
//!
//!     /// Type in a query and press `Enter`
//!     tab.type_str("WebKit")?.press_key("Enter")?;
//!
//!     /// We should end up on the WebKit-page once navigated
//!     tab.wait_for_element("#firstHeading")?;
//!     assert!(tab.get_url().ends_with("WebKit"));
//!
//!     /// Take a screenshot of the entire browser window
//!     let _jpeg_data = tab.capture_screenshot(
//!         Page::CaptureScreenshotFormatOption::Png,
//!         Some(75),
//!         None,
//!         true)?;
//!
//!     /// Take a screenshot of just the WebKit-Infobox
//!     let _png_data = tab
//!         .wait_for_element("#mw-content-text > div > table.infobox.vevent")?
//!         .capture_screenshot(ScreenshotFormat::PNG)?;
//!     Ok(())
//! }
//!
//! assert!(browse_wikipedia().is_ok());
//! ```

#![deny(clippy::pedantic)]
#![warn(renamed_and_removed_lints)]
#![allow(
clippy::unknown_clippy_lints,
clippy::module_name_repetitions,
clippy::doc_markdown, // a number of false positives here
clippy::default_trait_access, // fails on output of derive_builder
clippy::needless_pass_by_value, // would stop us creating and passing in LaunchOptions to browser in one statement
clippy::unreadable_literal, // not really applicable for timestamps
clippy::too_many_lines,
clippy::type_repetition_in_bounds,
clippy::used_underscore_binding
)]

#[macro_use]
extern crate derive_builder;
extern crate log;

pub use browser::{
    tab::{element::Element, Tab},
    Browser, LaunchOptions, LaunchOptionsBuilder,
};

#[cfg(feature = "fetch")]
pub use browser::FetcherOptions;

pub mod browser;
pub mod types;
pub mod protocol;
pub mod util;

#[cfg(feature = "nightly")]
#[doc = include_str!("../README.md")]
#[allow(dead_code)]
type _READMETEST = ();
