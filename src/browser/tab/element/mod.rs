use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use anyhow::{Error, Result};

use thiserror::Error;

use log::*;

use crate::{browser::tab::point::Point, protocol::cdp::CSS::CSSComputedStyleProperty};
use crate::browser::tab::NoElementFound;

mod box_model;

use crate::util;
pub use box_model::{BoxModel, ElementQuad};

use crate::protocol::cdp::{Page,CSS, Runtime, DOM};

#[derive(Debug, Error)]
#[error("Couldnt get element quad")]
pub struct NoQuadFound {}
/// A handle to a [DOM Element](https://developer.mozilla.org/en-US/docs/Web/API/Element).
///
/// Typically you get access to these by passing `Tab.wait_for_element` a CSS selector. Once
/// you have a handle to an element, you can click it, type into it, inspect its
/// attributes, and more. You can even run a JavaScript function inside the tab which can reference
/// the element via `this`.
pub struct Element<'a> {
    pub remote_object_id: String,
    pub backend_node_id: DOM::NodeId,
    pub node_id: DOM::NodeId,
    pub parent: &'a super::Tab,
    pub attrs: HashMap<String, String>
}

impl<'a> Debug for Element<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Element {}", self.backend_node_id)?;
        Ok(())
    }
}

impl<'a> Element<'a> {
    /// Using a 'node_id', of the type returned by QuerySelector and QuerySelectorAll, this finds
    /// the 'backend_node_id' and 'remote_object_id' which are stable identifiers, unlike node_id.
    /// We use these two when making various calls to the API because of that.
    pub fn new(parent: &'a super::Tab, node_id: DOM::NodeId) -> Result<Self> {
        if node_id == 0 {
            return Err(NoElementFound {}.into());
        }

        let node = parent
            .describe_node(node_id)
            .map_err(NoElementFound::map)?;

        let backend_node_id = node.backend_node_id;
        let remote_object_id = {
            let object = parent
                .call_method(DOM::ResolveNode {
                    backend_node_id: Some(backend_node_id),
                    node_id: None,
                    object_group: None,
                    execution_context_id: None,
                })?
                .object;

                object.object_id.expect("couldn't find object ID")
        };

        let mut attrs: HashMap<String, String> = HashMap::default();
        
        
        let attrs_vec = node.attributes.unwrap_or(vec![]);
        let tag = node.local_name;

        attrs.insert("tag".to_string(), tag);
        attrs.insert("value".to_string(), node.value.unwrap_or("".to_string()));
        if attrs_vec.len() > 0 {
            for i in (0..attrs_vec.len() - 1).step_by(2) {

                let k = attrs_vec[i].to_string();
                let v = attrs_vec[i+1].to_string();
                attrs.insert(k, v);
            }
        }
        
        Ok(Element {
            remote_object_id,
            backend_node_id,
            node_id,
            parent,
            attrs
        })
    }

    /// Returns the first element in the document which matches the given CSS selector.
    ///
    /// Equivalent to the following JS:
    ///
    /// ```js
    /// document.querySelector(selector)
    /// ```
    ///
    /// ```rust
    /// # use anyhow::Result;
    /// # // Awful hack to get access to testing utils common between integration, doctest, and unit tests
    /// # mod server {
    /// #     include!("../../../testing_utils/server.rs");
    /// # }
    /// # fn main() -> Result<()> {
    /// #
    /// use headless_chrome::Browser;
    ///
    /// let browser = Browser::default()?;
    /// let initial_tab = browser.new_tab()?;
    ///
    /// let file_server = server::Server::with_dumb_html(include_str!("../../../../tests/simple.html"));
    /// let containing_element = initial_tab.navigate_to(&file_server.url())?
    ///     .wait_until_navigated()?
    ///     .find_element("div#position-test")?;
    /// let inner_element = containing_element.find_element("#strictly-above")?;
    /// let attrs = inner_element.get_attributes()?.unwrap();
    /// assert_eq!(attrs["id"], "strictly-above");
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_element(&self, selector: &str) -> Result<Self> {
        self.parent
            .run_query_selector_on_node(self.node_id, selector)
    }

    pub fn find_element_by_xpath(&self, query: &str) -> Result<Element<'_>> {
        self.parent.get_document()?;

        self.parent
            .call_method(DOM::PerformSearch {
                query: query.to_string(),
                include_user_agent_shadow_dom: Some(true),
            })
            .and_then(|o| {
                Ok(self
                    .parent
                    .call_method(DOM::GetSearchResults {
                        search_id: o.search_id,
                        from_index: 0,
                        to_index: o.result_count,
                    })?
                    .node_ids[0])
            })
            .and_then(|id| {
                if id == 0 {
                    Err(NoElementFound {}.into())
                } else {
                    Ok(Element::new(self.parent, id)?)
                }
            })
    }

    /// Returns the first element in the document which matches the given CSS selector.
    ///
    /// Equivalent to the following JS:
    ///
    /// ```js
    /// document.querySelector(selector)
    /// ```
    ///
    /// ```rust
    /// # use anyhow::Result;
    /// # // Awful hack to get access to testing utils common between integration, doctest, and unit tests
    /// # mod server {
    /// #     include!("../../../testing_utils/server.rs");
    /// # }
    /// # fn main() -> Result<()> {
    /// #
    /// use headless_chrome::Browser;
    ///
    /// let browser = Browser::default()?;
    /// let initial_tab = browser.new_tab()?;
    ///
    /// let file_server = server::Server::with_dumb_html(include_str!("../../../../tests/simple.html"));
    /// let containing_element = initial_tab.navigate_to(&file_server.url())?
    ///     .wait_until_navigated()?
    ///     .find_element("div#position-test")?;
    /// let inner_divs = containing_element.find_elements("div")?;
    /// assert_eq!(inner_divs.len(), 5);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_elements(&self, selector: &str) -> Result<Vec<Self>> {
        self.parent
            .run_query_selector_all_on_node(self.node_id, selector)
    }

    pub fn find_elements_by_xpath(&self, query: &str) -> Result<Vec<Element<'_>>> {
        self.parent.get_document()?;
        self.parent
            .call_method(DOM::PerformSearch {
                query: query.to_string(),
                include_user_agent_shadow_dom: Some(true),
            })
            .and_then(|o| {
                Ok(self
                    .parent
                    .call_method(DOM::GetSearchResults {
                        search_id: o.search_id,
                        from_index: 0,
                        to_index: o.result_count,
                    })?
                    .node_ids)
            })
            .and_then(|ids| {
                ids.iter()
                    .filter(|id| **id != 0)
                    .map(|id| Element::new(self.parent, *id))
                    .collect()
            })
    }

    pub fn wait_for_element(&self, selector: &str) -> Result<Element<'_>> {
        self.wait_for_element_with_custom_timeout(selector, Duration::from_secs(3))
    }

    pub fn wait_for_xpath(&self, selector: &str) -> Result<Element<'_>> {
        self.wait_for_xpath_with_custom_timeout(selector, Duration::from_secs(3))
    }

    pub fn wait_for_element_with_custom_timeout(
        &self,
        selector: &str,
        timeout: std::time::Duration,
    ) -> Result<Element<'_>> {
        debug!("Waiting for element with selector: {:?}", selector);
        util::Wait::with_timeout(timeout).strict_until(
            || self.find_element(selector),
            Error::downcast::<NoElementFound>,
        )
    }

    pub fn wait_for_xpath_with_custom_timeout(
        &self,
        selector: &str,
        timeout: std::time::Duration,
    ) -> Result<Element<'_>> {
        debug!("Waiting for element with selector: {:?}", selector);
        util::Wait::with_timeout(timeout).strict_until(
            || self.find_element_by_xpath(selector),
            Error::downcast::<NoElementFound>,
        )
    }

    pub fn wait_for_elements(&self, selector: &str) -> Result<Vec<Element<'_>>> {
        debug!("Waiting for element with selector: {:?}", selector);
        util::Wait::with_timeout(Duration::from_secs(3)).strict_until(
            || self.find_elements(selector),
            Error::downcast::<NoElementFound>,
        )
    }

    pub fn wait_for_elements_by_xpath(&self, selector: &str) -> Result<Vec<Element<'_>>> {
        debug!("Waiting for element with selector: {:?}", selector);
        util::Wait::with_timeout(Duration::from_secs(3)).strict_until(
            || self.find_elements_by_xpath(selector),
            Error::downcast::<NoElementFound>,
        )
    }

    /// Moves the mouse to the middle of this element
    pub fn move_mouse_over(&self) -> Result<&Self> {
        self.scroll_into_view()?;
        let midpoint = self.get_midpoint()?;
        self.parent.move_mouse_to_point(midpoint)?;
        Ok(self)
    }

    pub fn click(&self) -> Result<&Self> {
        self.scroll_into_view()?;
        debug!("Clicking element {:?}", &self);

        self.call_js_fn("function() { this.click(); return 1; }", vec![], false)?
            .value.unwrap();


        /* let midpoint = self.get_midpoint()?;
        self.parent.click_point(midpoint)?; */
        if let Err(_) = self.parent.wait_until_navigated() {
            info!("[CLICK] Page load timeout..");
        }
        
        // MUST reload DOM in case page refreshed..
        self.parent.load_document();

        Ok(self)
    }

    pub fn type_into(&self, text: &str) -> Result<&Self> {
        self.click()?;

        debug!("Typing into element ( {:?} ): {}", &self, text);

        self.parent.type_str(text)?;

        Ok(self)
    }

    pub fn call_js_fn(
        &self,
        function_declaration: &str,
        args: Vec<serde_json::Value>,
        await_promise: bool,
    ) -> Result<Runtime::RemoteObject> {
        let mut args = args.clone();
        let result = self
            .parent
            .call_method(Runtime::CallFunctionOn {
                object_id: Some(self.remote_object_id.clone()),
                function_declaration: function_declaration.to_string(),
                arguments: args
                    .iter_mut()
                    .map(|v| {
                        Some(Runtime::CallArgument {
                            value: Some(v.take()),
                            unserializable_value: None,
                            object_id: None,
                        })
                    })
                    .collect(),
                return_by_value: Some(false),
                generate_preview: Some(true),
                silent: Some(false),
                await_promise: Some(await_promise),
                user_gesture: None,
                execution_context_id: None,
                object_group: None,
                throw_on_side_effect: None,
            })?
            .result;

        Ok(result)
    }

    pub fn focus(&self) -> Result<&Self> {
        self.scroll_into_view()?;
        self.parent.call_method(DOM::Focus {
            backend_node_id: Some(self.backend_node_id),
            node_id: None,
            object_id: None,
        })?;
        Ok(self)
    }

    /// Returns the inner text of an HTML Element. Returns an empty string on elements with no text.
    ///
    /// Note: .innerText and .textContent are not the same thing. See:
    /// https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText
    ///
    /// Note: if you somehow call this on a node that's not an HTML Element (e.g. `document`), this
    /// will fail.
    /// ```rust
    /// # use anyhow::Result;
    /// # fn main() -> Result<()> {
    /// #
    /// use headless_chrome::Browser;
    /// use std::time::Duration;
    /// let browser = Browser::default()?;
    /// let url = "https://web.archive.org/web/20190403224553/https://en.wikipedia.org/wiki/JavaScript";
    /// let inner_text_content = browser.new_tab()?
    ///     .navigate_to(url)?
    ///     .wait_for_element_with_custom_timeout("#Misplaced_trust_in_developers", Duration::from_secs(10))?
    ///     .get_inner_text()?;
    /// assert_eq!(inner_text_content, "Misplaced trust in developers");
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_inner_text(&self) -> Result<String> {
        let text: String = serde_json::from_value(
            self.call_js_fn("function() { return this.innerText }", vec![], false)?
                .value
                .unwrap(),
        )?;
        Ok(text)
    }

    /// Get the full HTML contents of the element.
    /// 
    /// Equivalent to the following JS: ```element.outerHTML```.
    pub fn get_content(&self) -> Result<String> {
        let html = self.
                call_js_fn("function() { return this.outerHTML }", vec![], false)?
                    .value
                    .unwrap();

        Ok(String::from(html.as_str().unwrap()))
    }

    pub fn get_computed_styles(&self) -> Result<Vec<CSSComputedStyleProperty>> {
        let styles = self
            .parent
            .call_method(CSS::GetComputedStyleForNode {
                node_id: self.node_id,
            })?
            .computed_style;

        Ok(styles)
    }

    pub fn get_description(&self) -> Result<DOM::Node> {
        let node = self
            .parent
            .call_method(DOM::DescribeNode {
                node_id: None,
                backend_node_id: Some(self.backend_node_id),
                depth: None,
                object_id: None,
                pierce: None,
            })?
            .node;
        Ok(node)
    }

    /// Capture a screenshot of this element.
    ///
    /// The screenshot is taken from the surface using this element's content-box.
    ///
    /// ```rust,no_run
    /// # use anyhow::Result;
    /// # fn main() -> Result<()> {
    /// #
    /// use headless_chrome::{protocol::page::ScreenshotFormat, Browser};
    /// let browser = Browser::default()?;
    /// let png_data = browser.new_tab()?
    ///     .navigate_to("https://en.wikipedia.org/wiki/WebKit")?
    ///     .wait_for_element("#mw-content-text > div > table.infobox.vevent")?
    ///     .capture_screenshot(ScreenshotFormat::PNG)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn capture_screenshot(
        &self,
        format: Page::CaptureScreenshotFormatOption,
    ) -> Result<Vec<u8>> {
        self.scroll_into_view()?;
        self.parent.capture_screenshot(
            format,
            Some(90),
            Some(self.get_box_model()?.content_viewport()),
            true,
        )
    }

    pub fn set_input_files(&self, file_paths: &[&str]) -> Result<&Self> {
        self.parent.call_method(DOM::SetFileInputFiles {
            files: file_paths.to_vec().iter().map(|v| v.to_string()).collect(),
            backend_node_id: Some(self.backend_node_id),
            node_id: None,
            object_id: None,
        })?;
        Ok(self)
    }

    /// Scrolls the current element into view
    ///
    /// Used prior to any action applied to the current element to ensure action is duable.
    pub fn scroll_into_view(&self) -> Result<&Self> {
        let result = self.call_js_fn(
            "async function() {
                if (!this.isConnected)
                    return 'Node is detached from document';
                if (this.nodeType !== Node.ELEMENT_NODE)
                    return 'Node is not of type HTMLElement';

                const visibleRatio = await new Promise(resolve => {
                    const observer = new IntersectionObserver(entries => {
                        resolve(entries[0].intersectionRatio);
                        observer.disconnect();
                    });
                    observer.observe(this);
                });

                if (visibleRatio !== 1.0)
                    this.scrollIntoView({
                        block: 'center',
                        inline: 'center',
                        behavior: 'instant'
                    });
                return false;
            }",
            vec![],
            true,
        )?;

        if result.Type == Runtime::RemoteObjectType::String {
            let error_text = result.value.unwrap().as_str().unwrap().to_string();
            return Err(ScrollFailed { error_text }.into());
        }

        Ok(self)
    }


    pub fn get_attributes(&self) -> Result<HashMap<String, String>> {
        // https://chromedevtools.github.io/devtools-protocol/tot/DOM/#method-getAttributes

        let mut attrs_map: HashMap<String, String> = HashMap::default();

        /* let attrs = self.parent
            .call_method(DOM::GetAttributes {
                node_id: self.node_id
            })?.attributes; */
        let node = self.get_description().unwrap(); //.local_name;
        
        
        let attrs = node.attributes.unwrap_or(vec![]);
        let tag = node.local_name;

        attrs_map.insert("tag".to_string(), tag);
        attrs_map.insert("value".to_string(), node.value.unwrap_or("".to_string()));
        for i in (0..attrs.len() - 1).step_by(2) {

            let k = attrs[i].to_string();
            let v = attrs[i+1].to_string();
            attrs_map.insert(k, v);
        }

        Ok(attrs_map)
    }

    /* pub fn get_attributes(&self) -> Result<Option<Vec<String>>> {
        let description = self.get_description()?;
        Ok(description.attributes)
    } */

    /// Get boxes for this element
    pub fn get_box_model(&self) -> Result<BoxModel> {
        let model = self
            .parent
            .call_method(DOM::GetBoxModel {
                node_id: None,
                backend_node_id: Some(self.backend_node_id),
                object_id: None,
            })?
            .model;
        Ok(BoxModel {
            content: ElementQuad::from_raw_points(&model.content),
            padding: ElementQuad::from_raw_points(&model.padding),
            border: ElementQuad::from_raw_points(&model.border),
            margin: ElementQuad::from_raw_points(&model.margin),
            width: model.width as f64,
            height: model.height as f64,
        })
    }

    pub fn get_midpoint(&self) -> Result<Point> {
        match self
            .parent
            .call_method(DOM::GetContentQuads {
                node_id: None,
                backend_node_id: Some(self.backend_node_id),
                object_id: None,
            })
            .and_then(|quad| {
                let raw_quad = quad.quads.first().unwrap();
                let input_quad = ElementQuad::from_raw_points(&raw_quad);

                Ok((input_quad.bottom_right + input_quad.top_left) / 2.0)
            }) {
            Ok(e) => return Ok(e),
            Err(_) => {
                let mut p = Point { x: 0.0, y: 0.0 };

                p = util::Wait::with_timeout(Duration::from_secs(20)).until(|| {
                    let r = self
                        .call_js_fn(
                            r#"
                    function() {
                        let rect = this.getBoundingClientRect();

                        if(rect.x != 0) {
                            this.scrollIntoView();
                        }

                        return this.getBoundingClientRect();
                    }
                    "#,
                            vec![],
                            false,
                        )
                        .unwrap();

                    let res = util::extract_midpoint(r);

                    match res {
                        Ok(v) => {
                            if v.x != 0.0 {
                                Some(v)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                })?;

                return Ok(p);
            }
        }
    }

    pub fn get_js_midpoint(&self) -> Result<Point> {
        let result = self.call_js_fn(
            "function(){return this.getBoundingClientRect(); }",
            vec![],
            false,
        )?;

        util::extract_midpoint(result)
    }
}

#[derive(Debug, Error)]
#[error("Scrolling element into view failed: {}", error_text)]
struct ScrollFailed {
    error_text: String,
}
