pub mod axis;
pub mod chart;
pub mod cursor;
pub mod data;
pub mod math;
pub mod prelude;
pub mod theme;

use crate::prelude::*;

use chart::Chart;
use leptos::callback::Callable;
use leptos::prelude::Callback;
use leptos::serde_json;
use nucleo::Utf32Str;
use nucleo::Utf32String;
use reactive_graph::effect::Effect;
use reactive_graph::owner::Owner;
use reactive_graph::wrappers::read::Signal;
use send_wrapper::SendWrapper;
use std::sync::Mutex;
use std::sync::OnceLock;
use thrs_layout as layout;

use crate::theme::Theme;


// ===================
// === AutoNewImpl ===
// ===================

pub trait AutoNewImpl {
    fn new() -> Self;
}

impl<T: Default> AutoNewImpl for T {
    fn new() -> Self {
        Self::default()
    }
}


// ===================
// === Buffer Data ===
// ===================

#[derive(Clone, Copy, Debug, Default)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

impl<T: Sub<T, Output = T>> Range<T> {
    pub fn len(self) -> T {
        self.end - self.start
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct ProjectionRange {
    #[deref]
    #[deref_mut]
    pub range: Range<Dec>,
    pub projection: Projection,
}

impl ProjectionRange {
    pub fn recompute(self) -> Self {
        let range = self.range;
        let scale = Dec::from(math::highest_power_of_ten(range.len()));
        let scale_100 = scale * Dec!(100);
        let origin = (self.start / scale_100).round() * scale_100;
        let projection = Projection { origin, scale };
        Self { range, projection }
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct ProjectedRange {
    #[deref]
    #[deref_mut]
    pub range: Range<f32>,
    pub projection: Projection,
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct View {
    pub xy: V2<ProjectionRange>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            xy: v2!(
            ProjectionRange {
                range: Range {
                    start: Dec!(0),
                    end: Dec!(0)
                },
                projection: Projection {
                    origin: Dec!(0),
                    scale: Dec!(1_000_000)
                }
            },
            ProjectionRange {
                range: Range {
                    start: Dec!(0),
                    end: Dec!(0)
                },
                projection: Projection {
                    origin: Dec!(0),
                    scale: Dec!(1_000)
                }
            }
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct ProjectedView {
    pub xy: V2<ProjectedRange>
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Projection {
    pub origin: Dec,
    pub scale: Dec
}

impl Projection {
    pub fn project(self, value: Dec) -> f32 {
        ((value - self.origin) / self.scale).into_f32()
    }

    pub fn unproject(self, value: f32) -> Dec {
        Dec::try_from(value).unwrap_or(Dec!(1)) * self.scale + self.origin
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self { origin: Dec!(0), scale: Dec!(1) }
    }
}

impl View {
    pub fn projections(self) -> V2<Projection> {
        self.xy.map(|t| t.projection)
    }

    pub fn recompute(self) -> Self {
        Self { xy: self.xy.map(|t| t.recompute()) }
    }

    pub fn recompute_and_check_if_changed(&mut self) -> bool {
        let old_projections = self.projections();
        *self = self.recompute();
        self.projections() != old_projections
    }
}

impl From<ProjectionRange> for ProjectedRange {
    fn from(t: ProjectionRange) -> Self {
        let projection = t.projection;
        let start = t.projection.project(t.range.start);
        let end = t.projection.project(t.range.end);
        let range = Range { start, end };
        Self { projection, range }
    }
}

impl From<ProjectedRange> for ProjectionRange {
    fn from(t: ProjectedRange) -> Self {
        let projection = t.projection;
        let start = t.projection.unproject(t.start);
        let end = t.projection.unproject(t.end);
        let range = Range { start, end };
        Self { range, projection }
    }
}

impl From<View> for ProjectedView {
    fn from(view: View) -> Self {
        Self { xy: view.xy.map(|t| (*t).into()) }
    }
}

impl From<ProjectedView> for View {
    fn from(view: ProjectedView) -> Self {
        Self { xy: view.xy.map(|t| (*t).into()) }
    }
}

impl ProjectedView {
    pub fn size(self) -> V2 {
        self.xy.map(|t| t.len())
    }

    pub fn mv(&mut self, delta: V2) {
        self.x.start += delta.x;
        self.x.end += delta.x;
        self.y.start += delta.y;
        self.y.end += delta.y;
    }
}


// ============
// === Plot ===
// ============

#[derive(Clone, Copy, Debug)]
pub struct Layers {
    plot_axis_background: rect::Mesh,
    plot_axis_highlight: rect::Mesh,
    plot_axis_ruler_label: text::Mesh,
}

impl Layers {
    const PLOT_AXIS_BACKGROUND_Z_INDEX: i32 = 10;
    const PLOT_AXIS_LABEL_Z_INDEX: i32 = 20;
    const PLOT_AXIS_HIGHLIGHT_Z_INDEX: i32 = 30;
    const PLOT_AXIS_HIGHLIGHT_LABEL_Z_INDEX: i32 = 40;

    pub fn new(
        ctx: p!(&<mut mesh, mut geometry, mut material, mut scene, font_atlas>Ctx),
        root_scene_handle: Ptr<Scene>
    ) -> Self {
        let plot_axis_background = rect::Mesh::new(p!(ctx));
        plot_axis_background.set_z_index(p!(ctx), Self::PLOT_AXIS_BACKGROUND_Z_INDEX);

        let plot_axis_highlight = rect::Mesh::new(p!(ctx));
        plot_axis_highlight.set_z_index(p!(ctx), Self::PLOT_AXIS_HIGHLIGHT_Z_INDEX);

        let plot_axis_highlight_label = text::Mesh::new(p!(ctx));
        plot_axis_highlight_label.set_z_index(p!(ctx), Self::PLOT_AXIS_HIGHLIGHT_LABEL_Z_INDEX);

        let root_scene = ctx.scene.get_mut(root_scene_handle);
        root_scene.add(plot_axis_highlight_label.mesh);
        root_scene.add(plot_axis_background.mesh);
        root_scene.add(plot_axis_highlight.mesh);

        Self { plot_axis_background, plot_axis_highlight, plot_axis_ruler_label: plot_axis_highlight_label }
    }
}


// =============
// === State ===
// =============

#[derive(Debug)]
pub struct DockedViewport {
    path: Rc<RefCell<layout::Path>>,
    _menu: layout::DockMenu,
    plot: Chart,
}

struct State {
    layout: layout::Layout,
    viewport_map: HashMap<layout::Path, DockedViewport>,
    focus: Option<layout::Path>,
    window_size: V2,
    device_pixel_ratio: f32,
}

impl State {
    fn init(&self) {
        web::window().with_or_warn(|web_window| {
            let web_document = &web_window.document;
            web_document.with_element_by_id("layout-root", |root| {
                self.layout.mount(root).warn();
            }).warn();
        });
    }
}

impl thrs::State for State {
    fn new(_ctx: p!(&<mut *>Ctx)) -> Result<Self> {
        let layout = layout::Layout::new()?;
        let viewport_map = default();
        let focus = default();
        let window_size = v2!(100.0);
        let device_pixel_ratio = 1.0;
        Ok(Self { layout, focus, viewport_map, window_size, device_pixel_ratio })
    }

    fn viewports(&self) -> impl Iterator<Item = Ptr<Viewport>> {
        self.viewport_map.values().map(|t| t.plot.viewport)
    }

    fn on_event(&mut self, ctx: p!(_ &<mut *>Ctx), event: window::Event) {
        match event {
            window::Event::RedrawRequested => {
                self.layout.refresh().warn();
                if let Some(events) = self.layout.output.take() {
                    self.on_layout_events(p!(ctx), events).warn()
                }
            }
            window::Event::KeyDown { key,..} => {
                if key == Key::AltLeft {
                    self.layout.input.send(layout::input::Event::EnableSplitMode);
                }
            }
            window::Event::KeyUp { key,..} => {
                if key == Key::AltLeft {
                    self.layout.input.send(layout::input::Event::DisableSplitMode);
                }
            }
            window::Event::Resized { device_pixel_ratio, size } => {
                self.window_size = size;
                self.device_pixel_ratio = device_pixel_ratio;
            },
            window::Event::Wheel { delta, client,..} => {
                if let Some(focus) = self.focus.as_ref()
                && let Some(viewport) = self.viewport_map.get_mut(focus) {
                    viewport.plot.on_wheel(p!(ctx), delta, client);
                }
            },
            _ => {}
        }
        for viewport in &mut self.viewport_map.values_mut() {
            viewport.plot.on_event(p!(ctx), event);
        }
    }
}

impl State {
    fn on_new_dock(&mut self, ctx: p!(_ &<mut *>Ctx), path: layout::Path) -> Result {
        let dock = self.layout.tree.get_dock_mut(&path)?;
        let rc_path = Rc::new(RefCell::new(path.clone()));
        let _menu = layout::DockMenu::new(&rc_path, &self.layout.input)?;
        dock.content.content_wrapper.append_child(&_menu)?;
        let plot = Chart::new(p!(ctx));
        self.viewport_map.insert(path, DockedViewport { path: rc_path, _menu, plot });
        Ok(())
    }

    fn viewport_mut(&mut self, path: &layout::Path) -> Result<&mut DockedViewport> {
        self.viewport_map.get_mut(path).with_context(|| format!("Invalid viewport path '{path:?}'."))
    }

    fn on_layout_events(&mut self, ctx: p!(_ &<mut *>Ctx), events: Vec<layout::output::Event>) -> Result {
        for event in events {
            match event {
                layout::output::Event::DockFocusChanged { focus } => {
                    self.focus = focus;
                },
                layout::output::Event::DockCons { path } => {
                    self.on_new_dock(p!(ctx), path)?;
                },
                layout::output::Event::DockSizeChanged { path, size } => {
                    let device_pixel_ratio = self.device_pixel_ratio;
                    self.viewport_mut(&path).map(|t| t.plot.set_size(p!(ctx), size, device_pixel_ratio))?;
                },
                layout::output::Event::DockPathChanged { old_path, new_path } => {
                    let viewport = self.viewport_map.remove(&old_path).context("Viewport not found.")?;
                    *viewport.path.borrow_mut() = new_path.clone();
                    self.viewport_map.insert(new_path, viewport);
                },
                layout::output::Event::DockOriginChanged { path, origin } => {
                    self.viewport_mut(&path).map(|t| t.plot.set_origin(p!(ctx), origin))?;
                },
                layout::output::Event::DockDrop { path } => {
                    self.viewport_map.remove(&path).context("Viewport not found.")?;
                },
            }
        }
        Ok(())
    }
}


// ============
// === Main ===
// ============

static CSS_REGISTRY: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn get_css_registry() -> &'static Mutex<Vec<String>> {
    CSS_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn register_global_css(s: &str) {
    match get_css_registry().lock() {
        Ok(mut vec) => vec.push(s.to_string()),
        Err(e) => warn!("Failed to lock CSS registry: {e}"),
    }
}

pub fn mount_registered_global_css() -> Result {
    let vec = get_css_registry().lock().map_err(|e| Error::msg(format!("{e}")))?;
    let css = vec.join("\n");
    let document = &web::window()?.document;
    let head = document.head().context("No head element.")?;
    let style_element = document.create_html_style_element()?;
    style_element.set_text_content(Some(&css));
    head.append_child(style_element.unchecked_as_web_sys_repr()).js_err()?;
    std::mem::forget(style_element);
    Ok(())
}

macro_rules! css {
    ($($ts:tt)*) => {
        #[ctor::ctor(anonymous)]
        fn ctor_register_global_css() {
            register_global_css($($ts)*);
        }
    };
}




/// /////////////////////


trait OptionCallbackRunner {
    type Args;
    fn run(&self, args: Self::Args);
}

impl<T> OptionCallbackRunner for Option<Callback<T>> {
    type Args = T;
    fn run(&self, args: T) {
        if let Some(f) = self.as_ref() {
            f.run(args)
        }
    }
}

pub mod dom {
    use super::*;
    pub type AnyElement = Rc<dyn AsRef<web::HtmlElement>>;
    pub type Elements = Vec<AnyElement>;
}




fn parse_style(properties: &str) -> Vec<(&str, &str)> {
    let mut out = vec![];
    for def in properties.split(';') {
        let def = def.trim();
        if def.is_empty() {
            continue;
        }
        if let Some((key, value)) = def.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            out.push((key, value));
        } else {
            // eprintln!("⚠️ invalid style definition (missing ':'): '{}'", def);
        }
    }
    out
}


// ===============
// === Element ===
// ===============

#[derive(Clone, Deref)]
pub struct Element(Rc<dyn DomRenderer>);

impl Element {
    pub fn new<T: DomRenderer + 'static>(component: T) -> Self {
        Self (Rc::new(component))
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element").finish()
    }
}

impl<T: DomRenderer + 'static> From<T> for Element {
    fn from(component: T) -> Self {
        Self::new(component)
    }
}


// ===================
// === IntoElement ===
// ===================

pub trait IntoElement {
    fn into_element(self) -> Element;
}

impl<T: DomRenderer + 'static> IntoElement for T {
    fn into_element(self) -> Element {
        Element::from(self)
    }
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

macro_rules! elements {
    ($($expr:expr),*$(,)?) => {
        vec![$(($expr).into_element()),*]
    };
}


// =================
// === Component ===
// =================

pub trait Component {
    fn render(&self) -> impl IntoElement;
}


// ===================
// === DomRenderer ===
// ===================

pub trait DomRenderer {
    fn render_dom(&self) -> dom::Elements;
}

impl<T: Component> DomRenderer for T {
    fn render_dom(&self) -> dom::Elements {
        render(&self.render().into_element())
    }
}

/// Specialized version of \[`DomRenderer`\] with specialization for \[`Element`\].
pub trait DomRendererSpec {
    fn render_dom_spec(&self) -> dom::Elements;
}

impl DomRendererSpec for Element {
    fn render_dom_spec(&self) -> dom::Elements {
        self.0.render_dom()
    }
}

impl<T: DomRenderer> DomRendererSpec for T {
    fn render_dom_spec(&self) -> dom::Elements {
        self.render_dom()
    }
}

fn render<T: DomRendererSpec>(view: &T) -> dom::Elements {
    let child_owner = Owner::new();
    let repr = child_owner.with(|| view.render_dom_spec());
    let repr2 = repr.clone();
    let repr2 = SendWrapper::new(repr2);
    Owner::on_cleanup(|| {
        drop(repr2);
        drop(child_owner);
    });
    repr
}


// =======================
// === Prim Components ===
// =======================

// === String ===

impl Component for &str {
    fn render(&self) -> impl IntoElement {
        Span::new().inner_html(*self)
    }
}

impl Component for &String {
    fn render(&self) -> impl IntoElement {
        self.as_str().render().into_element()
    }
}

impl Component for String {
    fn render(&self) -> impl IntoElement {
        self.as_str().render().into_element()
    }
}

// === Option ===

impl<T: Component> Component for Option<T> {
    fn render(&self) -> impl IntoElement {
        match self {
            Some(v) => v.render().into_element(),
            None => Empty.into_element()
        }
    }
}


// ===================
// === HtmlElement ===
// ===================

#[derive(Clone, Debug2)]
pub struct HtmlElement<Child=Element> {
    node_ref: Option<RwSignal<Option<SendWrapper<dom::AnyElement>>>>,
    id: Option<Signal<String>>,
    class: Option<Signal<String>>,
    style: Option<Signal<String>>,
    text: Option<Signal<String>>,
    children: Vec<Child>,
    inner_html: Option<String>,
    #[debug(skip)]
    listeners: Vec<Rc<dyn Fn(&web::Element) -> Result<web::WindowListenerHandle>>>
}

impl<Child> Default for HtmlElement<Child> {
    fn default() -> Self {
        Self {
            node_ref: default(),
            id: default(),
            class: default(),
            style: default(),
            text: default(),
            children: default(),
            inner_html: default(),
            listeners: default()
        }
    }
}

trait AsMutHtmlElement {
    type Child;
    fn as_mut_html_element(&mut self) -> &mut HtmlElement<Self::Child>;
}

impl<Child> AsMutHtmlElement for HtmlElement<Child> {
    type Child = Child;
    fn as_mut_html_element(&mut self) -> &mut HtmlElement<Self::Child> {
        self
    }
}

impl<T> AsMutHtmlElement for T where
T: DerefMut,
T::Target: AsMutHtmlElement {
    type Child = <<T as Deref>::Target as AsMutHtmlElement>::Child;
    fn as_mut_html_element(&mut self) -> &mut HtmlElement<Self::Child> {
        self.deref_mut().as_mut_html_element()
    }
}

impl<T: AsMutHtmlElement> HtmlElementOps for T {}
trait HtmlElementOps: Sized + AsMutHtmlElement {
    fn node_ref(mut self, node_ref: &NodeRef) -> Self {
        self.as_mut_html_element().node_ref = Some(node_ref.node);
        self
    }

    fn id(mut self, val: impl Into<Signal<String>>) -> Self {
        self.as_mut_html_element().id = Some(val.into());
        self
    }

    fn class(mut self, val: impl Into<Signal<String>>) -> Self {
        self.as_mut_html_element().class = Some(val.into());
        self
    }

    fn style(mut self, val: impl Into<Signal<String>>) -> Self {
        self.as_mut_html_element().style = Some(val.into());
        self
    }

    fn children(mut self, children: Vec<Self::Child>) -> Self {
        self.as_mut_html_element().children = children;
        self
    }

    fn inner_html(mut self, inner_html: impl Into<String>) -> Self {
        self.as_mut_html_element().inner_html = Some(inner_html.into());
        self
    }

    fn on<E: web::EventType>(self, f: impl FnMut(web::Arg<E>) + 'static) -> Self where
    web::Arg<E>: wasm_bindgen::convert::FromWasmAbi + 'static {
        self.on_with_options::<E>(f, Default::default())
    }

    fn on_with_options<E: web::EventType>
    (mut self, f: impl FnMut(web::Arg<E>) + 'static, options: web::EventListenerOptions) -> Self where
    web::Arg<E>: wasm_bindgen::convert::FromWasmAbi + 'static {
        let f2 = Rc::new(RefCell::new(f));
        let listener = Rc::new(move |t: &web::Element| {
            let f3 = f2.clone();
            t.add_event_listener_with_options::<E>(move |a| (f3.borrow_mut())(a), options)
        }) as Rc<dyn Fn(&web::Element) -> Result<web::WindowListenerHandle>>;
        self.as_mut_html_element().listeners.push(listener);
        self
    }
}

impl<Child: DomRendererSpec> HtmlElement<Child> {
    fn init(&self, repr: &dom::AnyElement) {
        if let Some(text) = &self.text {
            let text = *text;
            let repr = Rc::clone(repr);
            (*repr).as_ref().set_text_content(Some(&text.get_untracked()));
            Effect::new(move |_| (*repr).as_ref().set_text_content(Some(&text.get())));
        }

        if let Some(id) = &self.id {
            let id = *id;
            let repr = Rc::clone(repr);
            (*repr).as_ref().set_id(&id.get_untracked());
            Effect::new(move |_| (*repr).as_ref().set_id(&id.get()));
        }

        if let Some(class) = &self.class {
            let class = *class;
            let repr = Rc::clone(repr);
            (*repr).as_ref().set_class_name(&class.get_untracked());
            Effect::new(move |_| (*repr).as_ref().set_class_name(&class.get()));
        }

        if let Some(style) = &self.style {
            let style = *style;
            let repr = Rc::clone(repr);
            let repr_style = (*repr).as_ref().style();
            repr_style.set_properties(&parse_style(&style.get_untracked())).warn();
            Effect::new(move |_| { repr_style.set_properties(&parse_style(&style.get())).warn() });
        }

        let repr2 = Rc::clone(repr);
        let listeners = self.listeners.clone();
        Effect::new(move |_| {
            let listeners2 = listeners.clone();
            for listener in listeners2 {
                let handle = listener((*repr2).as_ref());
                Owner::on_cleanup(move || drop(handle));
            }
        });

        if let Some(inner_html) = &self.inner_html {
            (**repr).as_ref().set_inner_html(inner_html);
        }

        for child in &self.children {
            let this = (**repr).as_ref();
            for element in render(child) {
                this.append_child((*element).as_ref()).warn();
            }

        }

        if let Some(node_ref) = &self.node_ref {
            node_ref.set(Some(SendWrapper::new(Rc::clone(repr))))
        }
    }
}


// ===============
// === NodeRef ===
// ===============

#[derive(Clone, Copy, Debug, Deref)]
pub struct NodeRef {
    node: RwSignal<Option<SendWrapper<dom::AnyElement>>>,
}

impl Default for NodeRef {
    fn default() -> Self {
        Self { node: default() }
    }
}


// =============
// === Empty ===
// =============

#[derive(Clone, Copy, Debug, Default)]
pub struct Empty;

impl DomRenderer for Empty {
    fn render_dom(&self) -> dom::Elements {
        vec![]
    }
}


// ===========
// === Div ===
// ===========

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Div<Child=Element> {
    html_element: HtmlElement<Child>
}

impl<Child> Default for Div<Child> {
    fn default() -> Self {
        Self {
            html_element: default(),
        }
    }
}

impl<Child> Div<Child> {
    pub fn replace_children<Child2>(self, children: Vec<Child2>) -> Div<Child2> {
        let html_element = self.html_element;
        Div {
            html_element: HtmlElement {
                children,
                node_ref: html_element.node_ref,
                id: html_element.id,
                class: html_element.class,
                style: html_element.style,
                text: html_element.text,
                inner_html: html_element.inner_html,
                listeners: html_element.listeners,
            }
        }
    }
}

impl DomRenderer for Div {
    fn render_dom(&self) -> dom::Elements {
        let win = web::window().expect("Failed to access the browser window.");
        let repr = Rc::new(win.document.create_div().expect("Failed to create a 'div' element."))
        as dom::AnyElement;
        self.html_element.init(&repr);
        vec![repr]
    }
}


// ============
// === Span ===
// ============

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Span<Child=Element> {
    html_element: HtmlElement<Child>
}

impl<Child> Default for Span<Child> {
    fn default() -> Self {
        Self {
            html_element: default(),
        }
    }
}

impl DomRenderer for Span {
    fn render_dom(&self) -> dom::Elements {
        let win = web::window().expect("Failed to access the browser window.");
        let repr = Rc::new(win.document.create_span().expect("Failed to create a 'span' element."))
        as dom::AnyElement;
        self.html_element.init(&repr);
        vec![repr]
    }
}


// ===========
// === Div ===
// ===========

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct Input {
    #[deref]
    #[deref_mut]
    html_element: HtmlElement,
    placeholder: Option<Signal<String>>,
    value: Option<Signal<String>>,
    input_type: Option<Signal<String>>,
}

impl Input {
    pub fn placeholder(mut self, placeholder: impl Into<Signal<String>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    fn value(mut self, val: impl Into<Signal<String>>) -> Self {
        self.value = Some(val.into());
        self
    }

}

impl DomRenderer for Input {
    fn render_dom(&self) -> dom::Elements {
        let win = web::window().expect("Failed to access the browser window.");
        let repr = Rc::new(win.document.create_input().expect("Failed to create an 'input' element."));

        if let Some(placeholder) = &self.placeholder {
            let placeholder = *placeholder;
            let repr = Rc::clone(&repr);
            repr.set_placeholder(&placeholder.get_untracked());
            Effect::new(move |_| repr.set_placeholder(&placeholder.get()));
        }

        if let Some(value) = &self.value {
            let value = *value;
            let repr = Rc::clone(&repr);
            repr.set_value(&value.get_untracked());
            Effect::new(move |_| repr.set_value(&value.get()));
        }

        if let Some(input_type) = &self.input_type {
            let input_type = *input_type;
            let repr = Rc::clone(&repr);
            repr.set_value(&input_type.get_untracked());
            Effect::new(move |_| repr.set_type(&input_type.get()));
        }

        let repr = repr as dom::AnyElement;
        self.html_element.init(&repr);
        vec![repr]
    }
}


// ====================
// === TrafficLight ===
// ====================

#[derive(Clone, Copy, Debug, Default)]
pub struct TrafficLight;

impl Component for TrafficLight {
    fn render(&self) -> impl IntoElement {
        Div::new()
            .style("
                width: 14px;
                height: 14px;
                border-radius: 14px;
                background-color: rgba(255, 255, 255, 0.1);
            ")
    }
}


// =====================
// === TrafficLights ===
// =====================

#[derive(Clone, Copy, Debug, Default)]
pub struct TrafficLights;

impl Component for TrafficLights {
    fn render(&self) -> impl IntoElement {
        Div::new()
            .style("
                app-region: no-drag;
                display: flex;
                gap: 9px;
                padding: 17px;
                flex-shrink: 0;
            ").children(elements![
            TrafficLight,
            TrafficLight,
            TrafficLight,
        ])
    }
}


// ==============
// === Button ===
// ==============

css!("
    body {
        background-color: rgb(52 52 52 / 50%);
    }
");

css!("
    :root {
        --accent-color: rgba(236, 103, 19, 1);
        --button-height: 32px;
        --selected-opacity: 0.8;
        --not-selected-opacity: 0.3;
        --hint-opacity: 0.3;
    }

    .button {
        background-color: rgba(255,255,255,0.0);
        opacity: 1;
        transition: all 0.3s ease-in-out;
        border-radius: 100px;
        height: var(--button-height);

        &.hovered {
            // background-color: rgba(255,255,255,0.2);
        }

        &.selected {
            // background-color: rgba(255,255,255,0.2);
        }

        &.not-selected {
            opacity: var(--not-selected-opacity);
        }

        &.not-selected.hovered {
            opacity: 1;
        }

        &.pressed {
            // background-color: rgba(255,255,255,0.2);
        }
    }
");

#[derive(Clone, Debug, Default)]
pub struct Button {
    selected: Option<Signal<bool>>,
    on_click: Option<Callback<()>>,
    children: Vec<Element>
}

impl Button {
    pub fn children(mut self, children: Vec<Element>) -> Self {
        self.children = children;
        self
    }

    pub fn on_click(mut self, f: impl Into<Callback<()>>) -> Self {
        self.on_click = Some(f.into());
        self
    }

    pub fn selected(mut self, f: impl Into<Signal<bool>>) -> Self {
        self.selected = Some(f.into());
        self
    }
}

impl DomRenderer for Button {
    fn render_dom(&self) -> dom::Elements {
        let on_click = self.on_click;
        let hovered = RwSignal::new(false);
        let pressed = RwSignal::new(false);

        let selected = self.selected;
        let is_selected = move || selected.is_some_and(|s| s.get());
        let is_not_selected = move || selected.is_some_and(|s| !s.get());

        Effect::new({
            move |_| {
                let win = web::window().expect("Failed to access the browser window.");
                match win.add_event_listener::<web::MouseUp>(move |_| pressed.set(false)) {
                    Ok(handle) => Owner::on_cleanup(move || drop(handle)),
                    Err(e) => warn!("Failed to register MouseUp listener: {e}"),
                }
            }
        });

        let div =
        Div::new()
            .class("button-wrapper")
            .on::<web::MouseEnter>(move |_| hovered.set(true))
            .on::<web::MouseLeave>(move |_| {
                hovered.set(false);
                pressed.set(false);
            })
            .on::<web::MouseDown>(move |_| {
                pressed.set(true);
                on_click.run(());
            })
            .style("
                display: flex;
                align-items: center;

                height: var(--child-height);
                width: var(--child-width);
            ").children(elements![
            Div::new()
                .class(Signal::derive(move || {
                        let mut cls = "button".to_string();
                        if hovered.get() { cls.push_str(" hovered") }
                        if pressed.get() { cls.push_str(" pressed") }
                        if is_selected() { cls.push_str(" selected") }
                        if is_not_selected() { cls.push_str(" not-selected") }
                        cls
                    }))
                .style("
                    display: flex;
                    padding: 8px;
                    gap: 8px;
                    justify-content: center;
                    align-items: center;
                    z-index: 1;
                    min-height: 32px;
                    min-width: 16px;
                    text-wrap: nowrap;
                ").children(self.children.clone())
        ]);

        render(&div)
    }
}


// =============
// === Group ===
// =============

#[derive(Debug, Deref, DerefMut)]
pub struct Group<Child=Element> {
    div: Div<Child>,
}

impl<Child> Default for Group<Child> {
    fn default() -> Self {
        Self {
            div: default()
        }
    }
}

impl DomRenderer for Group<Element> {
    fn render_dom(&self) -> dom::Elements {
        self.div.render_dom()
    }
}

impl DomRenderer for Group<Button> {
    fn render_dom(&self) -> dom::Elements {
        let selected_ix = RwSignal::new(0);
        let children = self.div.children.iter().cloned().enumerate().map(|(ix, button)| {
            let old_on_click = button.on_click;
            let button = button
                .on_click(move || {
                    old_on_click.run(());
                    selected_ix.set(ix);
                    log!("clicked {ix}!");
                })
                .selected(Signal::derive(move || selected_ix.get() == ix));
            Element::new(button)
        }).collect();
        let div = self.div.clone().replace_children(children);
        div.render_dom()
    }
}


// =======================
// === UniversalSearch ===
// =======================

css!("
    .smooth-shadow {
        box-shadow: 0px 0px 1px rgba(3, 7, 18, 0.02),
            0px 0px 4px rgba(3, 7, 18, 0.03),
            0px 0px 9px rgba(3, 7, 18, 0.05),
            0px 0px 15px rgba(3, 7, 18, 0.06),
            0px 0px 24px rgba(3, 7, 18, 0.08);
    }
");

css!("
    input::placeholder { color: var(--color-placeholder); }
");


pub trait FuzzyMatch {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32>;
}


// =============
// === Entry ===
// =============

#[derive(Clone, Debug)]
pub enum Action {
    None,
    Submenu(Vec<Entry>)
}

// #[derive(Clone, Debug, Deref, DerefMut)]
// pub struct Entry {
//     #[deref]
//     #[deref_mut]
//     entry: EntryBase<AnyEntry>,
// }
//
// impl Entry {
//     pub fn new(entry: EntryBase<AnyEntry>) -> Self {
//         Self { entry }
//     }
// }

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Entry<T=AnyEntry> {
    label: Searchable<String>,
    #[deref]
    #[deref_mut]
    entry: T,
    action: Action,
}

impl<T> Entry<T> {
    pub fn new(label: impl Into<String>, entry: T, action: Action) -> Self {
        let label = Searchable::new(label);
        Self { label, entry, action }
    }
}

impl<T: FuzzyMatch> FuzzyMatch for Entry<T> {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32> {
        let mut depth = self.label.string.chars().filter(|c| *c == '/').count() as u32;
        if self.label.string.ends_with('/') {
            depth -= 1;
        }
        // We prefer label matches over entry matches.
        let label_match_multiplier = 10;
        let label_match = self.label
            .fuzzy_match(matcher, needle)
            .map(|t| (t + (20 - depth) * 1000) * label_match_multiplier);
        let entry_match = self.entry.fuzzy_match(matcher, needle);
        label_match.max(entry_match)
    }
}

#[derive(Clone, Debug, From)]
pub enum AnyEntry {
    Tool(Tool),
    Setting(Setting),
}

impl FuzzyMatch for AnyEntry {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32> {
        match self {
            Self::Tool(tool) => tool.fuzzy_match(matcher, needle),
            Self::Setting(setting) => setting.fuzzy_match(matcher, needle),
        }
    }
}

impl Component for Entry<AnyEntry> {
    fn render(&self) -> impl IntoElement {
        match &self.entry {
            // FIXME: CLONES
            AnyEntry::Tool(tool) => Entry { label: self.label.clone(), entry: tool, action: self.action.clone() }
                .render()
                .into_element(),
            AnyEntry::Setting(setting) => Entry {
                label: self.label.clone(),
                entry: setting,
                action: self.action.clone()
            }
                .render()
                .into_element(),
        }
    }
}


// ============
// === Tool ===
// ============

#[derive(Clone, Copy, Debug)]
pub struct Tool {
    icon: &'static str,
}

impl Tool {
    fn new(icon: &'static str) -> Self {
        Self {
            icon,
        }
    }
}

impl FuzzyMatch for Tool {
    fn fuzzy_match(&mut self, _matcher: &mut nucleo::Matcher, _needle: Utf32Str<'_>) -> Option<u32> {
        None
    }
}

css!("
    .searchable-text {
        & .matched {
            color: var(--accent-color);
        }
    }
");

impl Component for Entry<&Tool> {
    fn render(&self) -> impl IntoElement {
        Div::new()
            .style("
                display: flex;
                align-items: center;
                gap: 8px;
            ")
            .children(
            elements![
                Div::new().inner_html(self.icon),
                self.label.render()
            ]
            )
    }
}

impl<T> Component for Searchable<T> {
    fn render(&self) -> impl IntoElement {
        let chars = self.string.chars().skip(self.char_skip).collect::<Vec<_>>();
        let len = chars.len();
        let indices = &self.matched_indices;
        let mut non_matched_start = 0;
        let mut ix = 0;
        let mut out = vec![];
        loop {
            if ix >= indices.len() {
                let match_len = len - non_matched_start;
                if match_len > 0 {
                    out.push((match_len, false));
                }
                break
            }
            let matched_start: usize = indices[ix] as usize;
            if matched_start > non_matched_start {
                out.push((matched_start - non_matched_start, false))
            }
            ix += 1;

            let mut match_len = 1;
            loop {
                if ix >= indices.len() { break }
                if indices[ix] as usize != matched_start + match_len { break }
                match_len += 1;
                ix += 1;
            }
            non_matched_start = matched_start + match_len;
            out.push((match_len, true));
        }


        let mut chars_iter = chars.into_iter();

        let spans = out.into_iter().map(|(len, is_matched)| {
            let str = chars_iter.by_ref().take(len).collect::<String>();
            Element::new(
            Span::new()
                .class(if is_matched { "matched" } else { "unmatched" })
                .children(elements![str])
            )
        }).collect::<Vec<_>>();

        Span::new().class("searchable-text").children(spans)
    }
}


// ===============
// === Setting ===
// ===============

#[derive(Clone, Debug)]
pub struct Setting {
    icon: &'static str,
    value: SettingValue
}

impl Setting {
    fn new(icon: &'static str, value: impl Into<SettingValue>) -> Self {
        Self {
            icon,
            value: value.into()
        }
    }
}

#[derive(Clone, Debug)]
pub enum SettingValue {
    String(Searchable<String>),
    Usize(Searchable<usize>),
    Px(Searchable<usize>),
}

impl SettingValue {
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(Searchable::new(value))
    }

    pub fn usize(value: usize) -> Self {
        Self::Usize(Searchable::new(value))
    }

    pub fn px(value: usize) -> Self {
        Self::Px(Searchable::new(value))
    }
}

impl From<&str> for SettingValue {
    fn from(value: &str) -> Self {
        Self::String(Searchable::new(value))
    }
}

impl From<usize> for SettingValue {
    fn from(value: usize) -> Self {
        Self::Usize(Searchable::new(value))
    }
}

impl FuzzyMatch for SettingValue {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32> {
        match self {
            Self::String(val) => val.fuzzy_match(matcher, needle),
            Self::Usize(val) => val.fuzzy_match(matcher, needle),
            Self::Px(val) => val.fuzzy_match(matcher, needle),
        }
    }
}

impl Component for SettingValue {
    fn render(&self) -> impl IntoElement {
        let value = match self {
            Self::String(val) => &val.string,
            Self::Usize(val) => &val.string,
            Self::Px(val) => &val.string,
        };
        let suffix = matches!(self, Self::Px(_)).then(|| "px");
        Div::new()
            .style("
                height: 100%;
                display: flex;
                align-items: center;
                gap: 2px;
            ")
            .children(elements![
                Input::new()
                    .class("allow-focus")
                    .style("
                        height: 100%;
                        width: 60px;
                        background: transparent;
                        border: none;
                        outline: none;
                        font: inherit;
                        padding-left: 6px;
                        padding-right: 6px;
                        text-align: right
                    ")
                    .value(value.as_str()),
                suffix
            ])
    }
}

impl FuzzyMatch for Setting {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32> {
        self.value.fuzzy_match(matcher, needle)
    }
}

css!("
    input:focus {
        box-shadow: 0px 0px 0px 1px var(--accent-color);
        border-radius: 4px;
    }
");

impl Component for Entry<&Setting> {
    fn render(&self) -> impl IntoElement {
        Div::new()
            .style("
                height: 100%;
                display: flex;
                width: 100%;
                justify-content: space-between;
            ")
            .children(
            elements![
                Div::new()
                    .style("
                        height: 100%;
                        display: flex;
                        align-items: center;
                        gap: 8px;
                    ")
                    .children(
                    elements![
                        Div::new().inner_html(self.icon),
                        self.label.render()
                    ]
                ),
                self.value.render()
            ]
            )
    }
}


// ==================
// === Searchable ===
// ==================

#[derive(Clone, Debug, Deref)]
pub struct Searchable<T> {
    #[deref]
    pub value: T,
    pub string: String,
    pub search_string: String,
    pub search_string_utf32: Utf32String,
    pub len: u32,
    pub matched_indices: Vec<u32>,
    pub char_skip: usize,
}

impl<T: Display> Searchable<T> {
    pub fn new(value: impl Into<T>) -> Self {
        let value = value.into();
        let string = format!("{value}");
        let search_string = string.to_lowercase();
        let search_string_utf32 = search_string.clone().into();
        let matched_indices = default();
        let len = string.len() as u32;
        let char_skip = 0;
        Self { value, string, search_string, search_string_utf32, len, matched_indices, char_skip }
    }
}

impl<T: Display> From<T> for Searchable<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> FuzzyMatch for Searchable<T> {
    fn fuzzy_match(&mut self, matcher: &mut nucleo::Matcher, needle: Utf32Str<'_>) -> Option<u32> {
        self.matched_indices.clear();
        let haystack = self.search_string_utf32.slice(self.char_skip..);
        let result = matcher.fuzzy_indices(haystack, needle, &mut self.matched_indices);
        // We prioritize short matches. u16::MAX = 65_535, u32::MAX = 4_294_967_295
        let priority_mul = 1000;
        result.map(|t| ((t as u32) * priority_mul).saturating_sub(self.len))
    }
}


// ======================
// === GlobalDatabase ===
// ======================

thread_local! {
    pub static GLOBAL_DATABASE: Rc<RefCell<GlobalDatabase >> = init_global_database();
}

pub fn with_global_database_mut<T>(f: impl FnOnce(&mut GlobalDatabase) -> T) -> T {
    GLOBAL_DATABASE.with(|global| f(&mut global.borrow_mut()))
}

#[derive(Debug, Default)]
pub struct GlobalDatabase {
    pub entries: Vec<Entry>,
}

fn init_global_database() -> Rc<RefCell<GlobalDatabase>> {
    Rc::new(RefCell::new(GlobalDatabase {
        entries: vec![
            // Entry::new("Tools", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::Submenu(vec![
            Entry::new("tools /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("tools / trend_line", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("tools / ray", AnyEntry::Tool(Tool::new(ferris_trader_icons::RAY)), Action::None),
            Entry::new("tools / extended_line", AnyEntry::Tool(Tool::new(ferris_trader_icons::EXTENDED_LINE)), Action::None),
            Entry::new("tools / horizontal_line", AnyEntry::Tool(Tool::new(ferris_trader_icons::HORIZONTAL_LINE)), Action::None),
            Entry::new("tools / vertical_line", AnyEntry::Tool(Tool::new(ferris_trader_icons::VERTICAL_LINE)), Action::None),
            Entry::new("tools / cross_line", AnyEntry::Tool(Tool::new(ferris_trader_icons::CROSS_LINE)), Action::None),
            // ])),


            Entry::new("settings /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("settings / window_width", AnyEntry::Setting(Setting::new(ferris_trader_icons::WINDOW_WIDTH, SettingValue::px(1350))), Action::None),
            Entry::new("settings / window_height", AnyEntry::Setting(Setting::new(ferris_trader_icons::WINDOW_HEIGHT, SettingValue::px(850))), Action::None),
            Entry::new("settings / gui_server", AnyEntry::Setting(Setting::new(ferris_trader_icons::GUI_SERVER, "spawn")), Action::None),
            Entry::new("settings / exchanges /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("settings / exchanges / binance /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("settings / exchanges / binance / api_key", AnyEntry::Setting(Setting::new(ferris_trader_icons::KEY, "spawn")), Action::None),

            Entry::new("indicators /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("code /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
            Entry::new("actions /", AnyEntry::Tool(Tool::new(ferris_trader_icons::TREND_LINE)), Action::None),
        ]
    }))
}

// opacity: var(--not-selected-opacity);
// & .row.hovered {
//             opacity: var(--selected-opacity);
//         }
css!("
    .universal-search {
        & .row {
            opacity: 0.8;
        }
        & .row.hovered {
            opacity: 1;
            background-color: rgba(255, 255, 255, 0.5);
        }
        & .row.selected {
            opacity: 1;
            background-color: rgba(255, 255, 255, 1.0);
        }
    }

    .breadcrumbs {
        & .not-selected {
            opacity: 0.3;
        }
    }
");

#[derive(Clone, Copy, Debug, Default)]
pub struct UniversalSearch;

fn normalize_input(input: &str) -> String {
    let had_trailing_space = input.chars().last().is_some_and(|c| c.is_whitespace());
    let core = input.trim();
    let mut out = String::with_capacity(input.len());
    let mut prev_space = false;
    for (_, c) in core.char_indices() {
        match c {
            '/' => {
                if !out.is_empty() && !prev_space {
                    out.push(' ');
                }
                out.push('/');
                out.push(' ');
                prev_space = true;
            }
            c if c.is_whitespace() => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            _ => {
                out.push(c);
                prev_space = false;
            }
        }
    }
    let mut out = out.trim().to_string();
    if had_trailing_space {
        out.push(' ');
    }
    out
}

/// Returns everything up to and including the last `/`. If a space follows that `/`, it is included.
///
/// Returns an empty string if no `/` is present.
///
///     assert_eq!(base_dir("foo / bar / baz"), "foo / bar / ");
///     assert_eq!(base_dir("foo"), "");
fn base_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => {
            let end = i + 1;
            if path.as_bytes().get(end) == Some(&b' ') {
                &path[..end + 1]
            } else {
                &path[..end]
            }
        }
        None => "",
    }
}

fn remove_last_segment(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');

    if let Some(pos) = trimmed.rfind('/') {
        path[..=pos].to_string()
    } else {
        String::new()
    }
}

impl DomRenderer for UniversalSearch {
    fn render_dom(&self) -> dom::Elements {
        let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT.match_paths());

        let needle = RwSignal::new(String::new());
        let selected_index = RwSignal::new(0);
        let hovered_index: RwSignal<Option<usize>> = default();
        let breadcrumbs_hover_ix: RwSignal<Option<usize>> = default();

        let rows_ref = NodeRef::new();
        let input_ref = NodeRef::new();
        let input_overlay_ref = NodeRef::new();

        let current_rows: RwSignal<Vec<Entry>> = RwSignal::new(vec![]);
        let _scope: RwSignal<Vec<String>> = RwSignal::new(vec![]);

        let _rows_padding = 8;

        let mut prev_value = String::new();
        let on_input = move |target: web::web::HtmlInputElement, data: &str| {
            let ix = selected_index.get_untracked();
            selected_index.set(0);
            let value = target.value();

            log!(">>> {:?}", data);

            let new_value = match data {
                "/" => {

                    let rows = current_rows.get_untracked();
                    if let Some(row) = rows.get(ix) {
                        let last_matched_ix = row.label.matched_indices
                            .last()
                            .copied()
                            .unwrap_or(0) as usize + row.label.char_skip;
                        let max_depth = row.label.string.chars().filter(|&c| c == '/').count();
                        let new_depth = row.label.string
                            .as_str()
                            .chars()
                            .take(last_matched_ix)
                            .filter(|&c| c == '/')
                            .count() + 1;
                        let new_depth_clamped = Ord::min(new_depth, max_depth);
                        let mut new_value = row.label.string
                            .as_str()
                            .split('/')
                            .take(new_depth_clamped)
                            .collect::<Vec<_>>()
                            .join("/");
                        new_value.push('/');
                        new_value.push(' ');
                        new_value
                    } else {
                        prev_value.clone()
                    }
                }
                ". " => {
                    prev_value.clone()
                }
                _ => value
            };
            let normalized = normalize_input(&new_value);
            prev_value.clone_from(&normalized);
            needle.set(normalized);
        };

        let on_input = Rc::new(RefCell::new(on_input));
        let on_input2 = on_input.clone();
        let on_input3 = on_input.clone();

        Effect::new(move |_| {
            let needle_str = needle.get();
            if let Some(r) = input_ref.get_untracked() {
                let target = (**r).as_ref().unchecked_as_web_sys_repr().clone();
                if let Ok(input) = target.dyn_into::<web::web::HtmlInputElement>() {
                    input.set_value(&needle_str);
                } else {
                    warn!("Failed to cast element to HtmlInputElement.");
                }
            }
        });

        Effect::new(move |_| {
            let needle_lower = needle.get().to_lowercase();
            let needle_str = needle_lower.trim();
            let needle_utf32 = Utf32String::from(needle_str);
            // log!("needle: {:?}", needle_str);
            let _is_folder_root = needle_str.is_empty() || needle_str.ends_with('/');
            let _needle_depth = needle_str.chars().filter(|c| *c == '/').count();
            let rows_container_node = with_global_database_mut(|db| {
                let base = base_dir(needle_str);
                let base_len = base.chars().count();
                // log!("base: {:?}", base);

                let mut matches =
                db.entries.iter_mut().filter(|entry| {
                    let search_str = &entry.label.search_string;
                    let correct_prefix = search_str.starts_with(base);
                    let not_same = search_str != base;
                    correct_prefix && not_same
                }).map(|entry| {
                    entry.label.char_skip = base_len;
                    let result = entry.fuzzy_match(&mut matcher, needle_utf32.slice(base_len..));
                    (result, entry)
                }).filter(|(_result, entry)| {
                    let last_match = entry.label.matched_indices.last().copied().unwrap_or(0) as usize + entry.label.char_skip;
                    let last_slash = entry.label.string[0..entry.label.string.len() - 1].rfind('/').map_or(
                    0,
                        |i| i + 1
                    );
                    last_slash <= last_match
                })
                    .filter_map(|(result, entry)| result.map(|r| (r, entry)))
                    .collect::<Vec<_>>();

                matches.sort_by_key(|(a, _c)| *a);
                matches.reverse();

                let rows = matches.iter().enumerate().map(|(ix, (_i, entry))| {
                    let on_input4 = on_input3.clone();
                    Element::new(Div::new()
                        .class(Signal::derive(move || {
                            let mut cls = "row".to_string();
                            if selected_index.get() == ix { cls.push_str(" selected") }
                            if hovered_index.get() == Some(ix) { cls.push_str(" hovered") }
                            cls
                        }))
                        .style("
                            display: flex;
                            height: calc(1px * var(--theme-row-height));
                            padding-left: 10px;
                            padding-right: 10px;
                            border-radius: 14px;
                        ")
                        .children(elements![entry.render()])
                        .on::<web::MouseEnter>(move |_| hovered_index.set(Some(ix)))
                        .on::<web::MouseLeave>(move |_| hovered_index.set(None))
                        .on::<web::MouseDown>(move |_event| {
                            selected_index.set(hovered_index.get_untracked().unwrap_or(0));
                            if let Some(r) = input_ref.get_untracked() {
                                let target = (**r).as_ref().unchecked_as_web_sys_repr().clone();
                                if let Ok(input) = target.dyn_into::<web::web::HtmlInputElement>() {
                                    on_input4.borrow_mut()(input, "/");
                                } else {
                                    warn!("Failed to cast element to HtmlInputElement.");
                                }
                            }
                        })
                    )
                }).collect::<Vec<Element>>();


                current_rows.set(matches.iter().map(|t| t.1.clone()).collect());

                let rows_container = Div::new()
                    .class("rows-container")
                    .style("
                        width: 100%;
                        display: flex;
                        flex-direction: column-reverse;
                        padding: calc(1px * var(--theme-panel-padding));
                    ")
                    .children(rows);

                render(&rows_container)
            });

            if let Some(nr) = rows_ref.get() {
                (**nr).as_ref().set_inner_html("");
                for child in rows_container_node {
                    (**nr).as_ref().append_child((*child).as_ref()).warn();
                }
            }
        });


        Effect::new(move |_| {
            let needle_str = needle.get();

            let crumbs = needle_str.as_str().split('/').collect_vec();
            let crumbs_count = crumbs.len();
            let mut target = String::new();
            let items = crumbs.into_iter().enumerate().map(|(ix, part)| {
                let is_last = ix == crumbs_count - 1;
                let path = if is_last { part.to_string() } else { format!("{part}/") };
                target.push_str(&path);
                let current_target = target.clone();
                Element::new(
                Div::new()
                    .class(Signal::derive(move || {
                        if ix <= breadcrumbs_hover_ix.get().unwrap_or(1000) { "" } else { "not-selected" }
                    }))
                    .style("
                        display: flex;
                        align-items: center;
                        transition: all 0.3s ease-in-out;
                        cursor: pointer;
                    ")
                    .children(elements![path])
                    .on::<web::MouseEnter>(move |_| breadcrumbs_hover_ix.set(Some(ix)))
                    .on::<web::MouseLeave>(move |_| breadcrumbs_hover_ix.set(None))
                    .on::<web::MouseDown>(move |_| needle.set(format!("{current_target} ")))
                )
            }).collect_vec();
            let container = Div::new()
                .class("breadcrumbs")
                .style("
                    height: 100%;
                    display: flex;
                    white-space: pre-wrap;
                ")
                .children(items);
            let rendered = render(&container);
            if let Some(nr) = input_overlay_ref.get() {
                (**nr).as_ref().set_inner_html("");
                for child in rendered {
                    (**nr).as_ref().append_child((*child).as_ref()).warn();
                }
            }
        });


        let search_input = Div::new()
            .class("panel")
            .style("
                position: relative;
                display: flex;
                width: 800px;
            ")
            .children(elements![
                PanelBackground::default(),
                Div::new()
                    .class("content")
                    .style("
                        position: relative;
                        width: 100%;
                        height: 100%;
                        display: flex;
                        flex-direction: column;
                    ")
                    .children(elements![
                        Div::new()
                            .class("search-bar")
                            .style("
                                position: relative;
                                width: 100%;
                                height: 40px;
                                display: flex;
                            ")
                            .children(elements![
                                Div::new()
                                    .style("
                                        display: flex;
                                        height: 100%;
                                    ")
                                    .children(elements![
                                        Div::new()
                                            .style("
                                                display: flex;
                                                align-items: center;
                                                padding-left: 9px;
                                            ")
                                            .inner_html(ferris_trader_icons::DROPDOWN_ARROW),
                                        Div::new()
                                            .style("
                                                display: flex;
                                                align-items: center;
                                                padding-left: 3px;
                                            ")
                                            .inner_html(ferris_trader_icons::SEARCH),
                                    ]),
                                Div::new()
                                    .class("search-box")
                                    .style("
                                        position: relative;
                                        height: 100%;
                                        display: flex;
                                        flex-grow: 1;
                                    ")
                                    .children(elements![
                                        Div::new()
                                            .node_ref(&input_overlay_ref)
                                            .class("search-input-overlay")
                                            .style("
                                                position: absolute;
                                                left: 0;
                                                top: 0;
                                                height: 100%;
                                                padding-left: 8px;
                                                display: flex;
                                                align-items: center;
                                            "),
                                        Input::new()
                                            .node_ref(&input_ref)
                                            .class("search-input allow-focus")
                                            .style("
                                                flex-grow: 1;
                                                background: transparent;
                                                border: none;
                                                outline: none;
                                                box-shadow: none;
                                                font: inherit;
                                                padding-left: 8px;
                                                color: transparent;
                                                caret-color: var(--theme-text-color);
                                            ")
                                            .placeholder("Type to search ...")
                                            .on::<web::KeyDown>(move | event | {
                                                log!("keydown: {}", event.key());
                                                match event.key().as_str() {
                                                    "Enter" => {
                                                        if let Some(target) = event.target()
                                                        & &let Ok(input) = target.dyn_into::<web::web::HtmlInputElement>() {
                                                            on_input.borrow_mut()(input, "/");
                                                        } else {
                                                            warn!("Failed to get input element from event target.");
                                                        }
                                                    }
                                                    "ArrowUp" => selected_index.update(| ix | {
                                                            *ix + = 1;
                                                            let rows = current_rows.get_untracked();
                                                            if * ix> = rows.len() {
                                                                *ix - = 1;
                                                            }
                                                        }),
                                                    "ArrowDown" => selected_index.update(| t | {
                                                            *t = t.saturating_sub(1);
                                                        }),
                                                    "Backspace" => {
                                                        let needle_str = needle.get_untracked();
                                                        let needle_str_trimmed = needle_str.trim_end();
                                                        if needle_str_trimmed.ends_with('/') {
                                                            let new_needle = remove_last_segment(needle_str_trimmed);
                                                            let new_needle = if new_needle.is_empty() { new_needle } else {
                                                                format!("{new_needle}  ")
                                                            };
                                                            needle.set(new_needle);
                                                        }
                                                    }
                                                    key => {
                                                        log!("unknown key: {}", key);
                                                    }
                                                }
                                            })
                                            .on::<web::Input>(move | event | {
                                                if let Some(target) = event.target()
                                                & &let Ok(input) = target.dyn_into::<web::web::HtmlInputElement>() {
                                                    on_input2.borrow_mut()(input, event.data().as_deref().unwrap_or(""));
                                                } else {
                                                    warn!("Failed to get input element from event target.");
                                                }
                                            })
                                    ])

                            ]),
                    ])
            ]);

        let max_result_count = 15.5;
        let max_height = format!("calc(1px * {max_result_count} * var(--theme-row-height) + 1px * var(--theme-panel-padding))");

        let search_results = Div::new()
            .class("panel")
            .style("
                position: relative;
                display: flex;
                width: 800px;
                border-radius: 20px;
                overflow: hidden;
            ")
            .children(elements![
                PanelBackground::default(),
                Div::new()
                    .class("content")
                    .style(format!("
                        position: relative;
                        width: 100%;
                        max-height: {max_height};
                        overflow: scroll;
                        display: flex;
                        flex-direction: column-reverse;
                    "))
                    .children(elements![
                        // Div::new()
                        //     .class("header")
                        //     .style("
                        //         width:100%;
                        //         height:48px;
                        //         padding: 4px;
                        //     ").children(elements![
                        //         Group::default()
                        //             .style("
                        //                 display: flex;
                        //                 flex-grow: 1;
                        //                 --child-padding-horizontal: 2px;
                        //                 --child-padding-vertical: 0px;
                        //                 --child-height: 100%;
                        //                 --child-width: auto;
                        //             ")
                        //             .children(vec![
                        //                 Button::default()
                        //                     .children(elements!["Everything"]),
                        //                 Button::default()
                        //                     .children(elements!["Tools"]),
                        //                 Button::default()
                        //                     .children(elements!["Indicators"]),
                        //                 Button::default()
                        //                     .children(elements!["Code"]),
                        //                 Button::default()
                        //                     .children(elements!["Actions"]),
                        //                 Button::default()
                        //                     .children(elements!["Settings"]),
                        //             ])
                        //     ]),
                        Div::new()
                            .node_ref(&rows_ref)
                            .class("rows")
                            .style("
                                width: 100%;
                                display: flex;
                            ")
                    ])
            ]);


        let div = Div::new()
            .class("universal-search")
            .style("
                position: absolute;
                width: 100%;
                height: 100%;
                background-color: rgba(0, 0, 0, 0.4);
                z-index: 1000;
            ").children(elements![
            Div::new()
                .class("content")
                .style("
                    width: 100%;
                    position: absolute;
                    bottom: 64px;
                    display: flex;
                    gap: 4px;
                    align-items: center;
                    flex-direction: column;
                ")
                .children(elements![
                    search_results,
                    search_input,
                ])
        ]);
        render(&div)
    }
}


// =============================
// === BackgroundNoiseFilter ===
// =============================

#[derive(Clone, Copy, Debug, Default)]
pub struct BackgroundNoiseFilter;

impl DomRenderer for BackgroundNoiseFilter {
    fn render_dom(&self) -> dom::Elements {
        let svg = r#"
                <svg xmlns="http://www.w3.org/2000/svg" style="display:none">
                     <filter id="BackgroundNoiseFilter" x="0" y="0" width="100%" height="100%">
                         <feTurbulence
                             type="fractalNoise"
                             baseFrequency="1.3"
                             numOctaves="1"
                             stitchTiles="stitch"/>
                     </filter>
                 </svg>
            "#;
        render(&svg)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PanelBackground {
    selected: bool
}


impl DomRenderer for PanelBackground {
    fn render_dom(&self) -> dom::Elements {
        let _blur = blur_chain(8, 2, 5);
        let background_color = if self.selected { "rgba(255,255,255,1)" } else { "var(--theme-panel-background-color)" };
        let div = Div::new()
            .class("panel-background")
            .style("
                position: absolute;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                overflow: hidden;
                border-radius: 20px;
            ")
            .children(elements![
                Div::new()
                    .style(format!("
                        position: absolute;
                        top: -64px;
                        left: -64px;
                        width: calc(100% + 128px);
                        height: calc(100% + 128px);
                        background-color: {background_color};
                        backdrop-filter: blur(64px);
                    "))
            ]);
        // .children(elements![
        //     Div::new()
        //         .style("
        //             position: absolute;
        //             top: 0;
        //             left: 0;
        //             width: 100%;
        //             height: 100%;
        //             filter: url(#BackgroundNoiseFilter);
        //             opacity: 0.1;
        //             mix-blend-mode: plus-lighter;
        //         ")
        // ]);
        render(&div)
    }
}

fn blur_chain(start: u32, multiplier: u32, steps: usize) -> String {
    (0..steps).map(|i| start * multiplier.pow(i as u32)).map(|v| format!("blur({v}px)")).collect::<Vec<_>>().join(" ")
}


// ==============
// === DomApp ===
// ==============

#[derive(Clone, Copy, Debug, Default)]
pub struct DomApp;

impl DomRenderer for DomApp {
    fn render_dom(&self) -> dom::Elements {
        let _theme = Theme::get();
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
    let ctx = &mut app_state.registry;

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

    app_state.state.init();

    web_document.with_element_by_id("gl-root", |root| {
        let root_html = root.dyn_as_html_element()
            .expect("Element 'gl-root' is not an HtmlElement.");
        app.mount_to(root_html).warn();
    }).warn();

    drop(app_state);
    std::mem::forget(app);
}
