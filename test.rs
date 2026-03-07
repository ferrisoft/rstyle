pub mod axis;
pub mod theme;
pub mod math;
pub mod data;
pub mod prelude;
pub mod cursor;
pub mod chart;

use crate::prelude::*;

use thrs_layout as layout;

use chart::Chart;
use std::sync::{Mutex, OnceLock};
use send_wrapper::SendWrapper;

#[derive(Copy, Clone, Default, Debug)]
pub struct DomApp;

impl DomRenderer for DomApp {
fn render_dom( & self ) -> dom::Elements {
let _theme=Theme::get();
let div =
Div::new()
.class("app")
.style("
opacity: 1.0;
position: relative;
display: flex;
width: 100vw;
height: 100vh;
font-family: \"SF Pro Text\";
font-size: 12px;
color: var(--theme-text-color);
--color-placeholder: var(--theme-text-placeholder-color);
")
.children(elements![
BackgroundNoiseFilter,
UniversalSearch,
Div::new()
.class("content")
.style("
position: relative;
flex-grow: 1;
display: flex;
flex-direction: column;
")
.children(elements![
Div::new()
.class("menu-bar")
.style("
app-region: no-drag;
position: relative;
display: flex;
width: 100%;
height: 48px;
background-color: black;
flex-shrink: 0;
cursor: default;
").children(elements![
TrafficLights,
Group::default()
.style("
display: flex;
flex-grow: 1;
align-items: center;
--child-padding-horizontal: 2px;
--child-padding-vertical: 0px;
--child-height: 100%;
--child-width: auto;
")
.children(vec![
Button::default()
.children(elements![
Div::new().inner_html(ferris_trader_icons::CHART),
"foo"
]),
Button::default()
.children(elements![
Div::new().inner_html(ferris_trader_icons::CHART)
])
.on_click(|| log!("lol")),
])
]),
Div::new()
.class("body")
.style("
position: relative;
width: 100%;
flex-grow: 1;
")
.children(elements![
Div::new().id("layout-root"),
Div::new().id("gl-root")
]),
])
]);

render(&div)
}
}




pub mod universal_search {
thrs::message_bus::message! {
pub enum Event {
Open,
Close
}
}
}


#[wasm_bindgen(start)]
pub async fn main() {
log!("::: {}", serde_json::to_string(&universal_search::Event::Open)
.expect("Failed to serialize universal_search::Event."));

mount_registered_global_css().expect("Failed to mount global CSS styles.");
any_spawner::Executor::init_wasm_bindgen().warn();

let web_window = web::window().expect("Failed to access the browser window.");
let web_document = &web_window.document;

let app = App::<State>::new().await.expect("Failed to initialize the application.");

let mut app_state = app.state.borrow_mut();
let ctx = &mut (*app_state).as_mut().expect("Application state was not initialized.").registry;

let _queue = ctx.message_bus.new_queue::<universal_search::Event>("universal_search");
ctx.shortcut_registry
.register(&ctx.message_bus, "shift", "universal_search: {\"type\":\"Open\"}")
.expect("Failed to register keyboard shortcut.");
ctx.propagate_shortcuts().warn();

let theme = ctx.theme.get::<Theme>();
theme.register_css_vars();

let owner = Owner::new();
owner.with(move || {
reactive_graph::owner::provide_context(theme);
for child in render(&DomApp) {
web_document.body.append_child((*child).as_ref())
.expect("Failed to append child element to document body.");
}
});
std::mem::forget(owner);

app_state.as_ref().expect("Application state was not initialized.").state.init();

web_document.with_element_by_id("gl-root", |root| {
let root_html = root.dyn_as_html_element()
.expect("Element 'gl-root' is not an HtmlElement.");
app.mount_to(root_html).warn();
}).warn();

drop(app_state);
std::mem::forget(app);
}
