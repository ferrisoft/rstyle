use crate::prelude::*;

use std::fmt::Debug;

use crate::Category;
use crate::candle::Ohlc;
use crate::candle::Ohlcv;

pub type Categories = &'static [Category];


// =================
// === Indicator ===
// =================

#[derive(Clone, Copy, Debug)]
pub struct Labels {
    pub full: &'static str,
    pub short: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct AlertSignal {
    pub label: &'static str,
    pub description: &'static str,
    pub enabled_by_default: bool,
}

pub trait Indicator: Sized {
    type Input: Copy;
    type Output: Copy + Debug;
    type Alerts: Copy + Debug + Default;
    type State: IndicatorState<Indicator = Self>;
    type Plot: IndicatorPlot<Indicator = Self>;
    fn categories() -> Categories;
    fn labels() -> Labels;
    fn alert_signals() -> &'static [AlertSignal] { &[] }
    fn plot_config() -> plot::Config {
        default()
    }
}

pub trait IndicatorAssoc {
    type Indicator: Indicator;
}

pub type IndicatorOf<T> = <T as IndicatorAssoc>::Indicator;


// ==================
// === StepResult ===
// ==================

#[derive(Clone, Copy, Debug)]
pub struct StepResult<T: Indicator> {
    pub output: Output<T>,
    pub alerts: Alerts<T>,
}


// ======================
// === IndicatorState ===
// ======================

/// Stateful per-bar computation. Generic over indicator config and I/O types.
///
/// `push_value` always returns both output and alert signals. Alerts are computed
/// inside `push_value` with full access to internal state. The engine caches
/// both output and alerts; the trading system reads alerts from cache,the
/// visualization layer reads only outputs.
pub trait IndicatorState: IndicatorAssoc {
    fn new(config: Self::Indicator) -> Self;
    fn push_value(&mut self, input: Input<Self::Indicator>) -> StepResult<Self::Indicator>;
    #[inline(always)]
    fn push_values(
        &mut self,
        input: &[Input<Self::Indicator>],
        results: &mut [StepResult<Self::Indicator>],
    ) {
        for (i, &val) in input.iter().enumerate() {
            results[i] = Self::push_value(self, val);
        }
    }
}

pub type Input<T> = <T as Indicator>::Input;
pub type Output<T> = <T as Indicator>::Output;
pub type Alerts<T> = <T as Indicator>::Alerts;
pub type State<T> = <T as Indicator>::State;


// ================
// === NoAlerts ===
// ================

/// Zero-sized alert type for indicators that don't define alert signals.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NoAlerts;


// ============
// === plot ===
// ============

pub mod plot {
    use super::*;

    /// Anything that can produce per-bar visual samples — both individual plot
    /// primitives (Line,Fill,Marker) and composite views.
    pub trait Component {
        type Sample: Copy + Default;
        fn new_sample(&self) -> Self::Sample;
    }

    /// `Sample<Line>` instead of `<Line as Component>::Sample`.
    pub type Sample<T> = <T as Component>::Sample;

    #[derive(Clone, Copy, Debug)]
    pub struct Config {
        pub overlay: bool,
        pub value_domain: ValueDomain,
        pub scale: Scale,
        pub behind_chart: bool,
        pub format: ValueFormat,
        pub precision: Option<u8>,
        pub suggested_timeframe: Option<Timeframe>,
        pub gap_fill: GapFill,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                overlay: false,
                value_domain: ValueDomain::default(),
                scale: Scale::default(),
                behind_chart: true,
                format: ValueFormat::default(),
                precision: None,
                suggested_timeframe: None,
                gap_fill: GapFill::default(),
            }
        }
    }
}

use plot::Component as _;


// =====================
// === IndicatorPlot ===
// =====================

pub trait IndicatorPlot: plot::Component + IndicatorAssoc {
    fn new() -> Self;
    fn sample(&self, ctx: &PlotContext<'_, Output<Self::Indicator>>) -> Self::Sample;
}


// =============
// === Color ===
// =============

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub alpha: f32,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, alpha: 1.0 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, alpha: f32) -> Self {
        Self { r, g, b, alpha }
    }

    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const ORANGE: Self = Self::rgb(255, 165, 0);
    pub const PURPLE: Self = Self::rgb(128, 0, 128);
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0.0);
}

impl Default for Color {
    fn default() -> Self { Self::RED }
}


// ===============
// === Display ===
// ===============

/// Bitflag controlling where a plot component's values are shown.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Display(u8);

impl Display {
    pub const PANE: Self = Self (0b0001);
    pub const STATUS_LINE: Self = Self (0b0010);
    pub const PRICE_SCALE: Self = Self (0b0100);
    pub const DATA_WINDOW: Self = Self (0b1000);
    pub const ALL: Self = Self (0b1111);
    pub const NONE: Self = Self (0b0000);

    pub fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
}

impl Default for Display {
    fn default() -> Self { Self::ALL }
}

impl std::ops::BitOr for Display {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self (self.0 | rhs.0) }
}

impl Sub for Display {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self (self.0 & !rhs.0) }
}


// ==============
// === Styles ===
// ==============

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MarkerShape {
    #[default]
    Circle,
    Diamond,
    Cross,
    XCross,
    Square,
    TriangleUp,
    TriangleDown,
}

/// Shape icons for conditional shape markers (`plotshape()` equivalent).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ShapeIcon {
    #[default]
    Circle,
    TriangleUp,
    TriangleDown,
    Cross,
    Diamond,
    Flag,
    ArrowUp,
    ArrowDown,
    Square,
    XCross,
    LabelUp,
    LabelDown,
}

/// Vertical placement of shapes relative to the bar.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ShapeLocation {
    #[default]
    AboveBar,
    BelowBar,
    Top,
    Bottom,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TextSize {
    Tiny,
    Small,
    #[default]
    Normal,
    Large,
    Huge,
}

/// Width of rectangular bars.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BarWidth {
    /// Thin bars — equivalent to `TradingView`'s `histogram` style.
    #[default]
    Thin,
    /// Wide bars filling the bar space — equivalent to `TradingView`'s `columns` style.
    Wide,
}

/// Baseline from which bars grow.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Baseline {
    #[default]
    /// Bars grow from the zero line.
    Zero,
    /// Bars grow from the bottom of the pane.
    Bottom,
    /// Bars grow from an arbitrary price level (e.g. 50.0 for RSI-centered histograms).
    Value(f64),
}

/// What value domain the indicator outputs live in.
///
/// Lets the engine make intelligent decisions about scale,overlay validity,and formatting.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ValueDomain {
    /// Output values are in the same units as price.
    #[default]
    Price,
    /// Output has a fixed bounded range (e.g. RSI 0-100).
    Bounded { min: f64, max: f64 },
    /// Output oscillates around zero with no fixed bounds (MACD,CCI).
    Unbounded,
    /// Output is in volume units.
    Volume,
    /// Output is percentage values.
    Percent,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Scale {
    #[default]
    None,
    Right,
    Left,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ValueFormat {
    #[default]
    Inherit,
    Price,
    Volume,
    Percent,
}

/// Rendering style for OHLC data.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CandleStyle {
    /// Filled body with wicks.
    #[default]
    Candlestick,
    /// Vertical line (high-low) with horizontal ticks for open (left) and close (right).
    OhlcBar,
}


// =================
// === Timeframe ===
// =================

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Timeframe {
    Second(u32),
    Minute(u32),
    Hour(u32),
    Day(u32),
    Week(u32),
    Month(u32),
}


// ===============
// === GapFill ===
// ===============

/// How resampled indicator outputs are displayed on a finer-grained chart.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GapFill {
    /// Repeat the last value across bars until a new value arrives (staircase display).
    #[default]
    ForwardFill,
    /// Show gaps between resampled values.
    Gaps,
}


// =================
// === Displaced ===
// =================

#[derive(Clone, Copy, Debug)]
pub struct Displaced<T> {
    pub value: T,
    pub offset: i32,
}

impl<T> Displaced<T> {
    pub fn none(value: T) -> Self { Self { value, offset: 0 } }
    pub fn forward(value: T, bars: u32) -> Self { Self { value, offset: bars as i32 } }
    pub fn backward(value: T, bars: u32) -> Self { Self { value, offset: -(bars as i32) } }
}

impl<T> From<T> for Displaced<T> {
    fn from(value: T) -> Self { Self::none(value) }
}


// ===================
// === PlotContext ===
// ===================

#[derive(Debug)]
pub struct PlotContext<'a, T> {
    pub start_time: i64,
    pub output: &'a [T],
    pub candles: &'a [Ohlcv],
    pub current_index: usize,
    pub total_count: usize,
}

impl<T: Copy> PlotContext<'_, T> {
    pub fn current_output(&self) -> T {
        self.output[self.current_index]
    }

    pub fn current_candle(&self) -> Ohlcv {
        self.candles[self.current_index]
    }

    /// Output from N bars ago (1-based:`prev_output(1)`=previous bar).
    pub fn prev_output(&self, n: usize) -> Option<T> {
        self.current_index.checked_sub(n).and_then(|i| self.output.get(i)).copied()
    }

    /// Candle from N bars ago (`prev_candle(0)`=current candle).
    pub fn prev_candle(&self, n: usize) -> Option<Ohlcv> {
        self.current_index.checked_sub(n).and_then(|i| self.candles.get(i)).copied()
    }

    pub fn is_first(&self) -> bool { self.current_index == 0 }
    pub fn is_last(&self) -> bool { self.current_index == self.total_count - 1 }
    pub fn available_history(&self) -> usize { self.output.len() }
}


// ============
// === Line ===
// ============

#[derive(Clone, Copy, Debug, Default)]
pub struct Line {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    pub interpolation: Interpolation,
    pub break_on_gaps: bool,
    pub offset: i32,
    pub track_price: bool,
    pub default_sample: LineSample,
}

#[derive(Clone, Copy, Debug)]
pub struct LineSample {
    pub value: Option<f64>,
    pub color: Color,
    pub width: u8,
    pub style: LineStyle,
}

impl Default for LineSample {
    fn default() -> Self {
        Self { value: None, color: Color::BLUE, width: 1, style: LineStyle::Solid }
    }
}

impl LineSample {
    pub fn set_value(&mut self, val: f64) { self.value = Some(val); }
    pub fn hide(&mut self) { self.value = None; }
}

impl Line {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            interpolation: Interpolation::Linear,
            break_on_gaps: false,
            offset: 0,
            track_price: false,
            default_sample: LineSample::default(),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.default_sample.color = color; self }
    pub fn set_width(mut self, width: u8) -> Self { self.default_sample.width = width; self }
    pub fn set_style(mut self, style: LineStyle) -> Self { self.default_sample.style = style; self }
    pub fn set_offset(mut self, offset: i32) -> Self { self.offset = offset; self }
    pub fn set_interpolation(mut self, interp: Interpolation) -> Self { self.interpolation = interp; self }
    pub fn set_break_on_gaps(mut self, brk: bool) -> Self { self.break_on_gaps = brk; self }
    pub fn set_track_price(mut self, track: bool) -> Self { self.track_price = track; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Line {
    type Sample = LineSample;
    fn new_sample(&self) -> LineSample { self.default_sample }
}


// ==============
// === Marker ===
// ==============

#[derive(Clone, Copy, Debug, Default)]
pub struct Marker {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    pub shape: MarkerShape,
    pub offset: i32,
    pub default_sample: MarkerSample,
}

#[derive(Clone, Copy, Debug)]
pub struct MarkerSample {
    pub value: Option<f64>,
    pub color: Color,
    pub size: f32,
}

impl Default for MarkerSample {
    fn default() -> Self {
        Self { value: None, color: Color::BLUE, size: 4.0 }
    }
}

impl MarkerSample {
    pub fn set_value(&mut self, val: f64) { self.value = Some(val); }
    pub fn hide(&mut self) { self.value = None; }
}

impl Marker {
    pub fn new(label: &'static str, shape: MarkerShape) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            shape,
            offset: 0,
            default_sample: MarkerSample::default(),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.default_sample.color = color; self }
    pub fn set_size(mut self, size: f32) -> Self { self.default_sample.size = size; self }
    pub fn set_offset(mut self, offset: i32) -> Self { self.offset = offset; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Marker {
    type Sample = MarkerSample;
    fn new_sample(&self) -> MarkerSample { self.default_sample }
}


// ============
// === Bars ===
// ============

#[derive(Clone, Copy, Debug, Default)]
pub struct Bars {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    pub bar_width: BarWidth,
    pub baseline: Baseline,
    pub offset: i32,
    pub default_sample: BarsSample,
}

#[derive(Clone, Copy, Debug)]
pub struct BarsSample {
    pub value: Option<f64>,
    pub color: Color,
}

impl Default for BarsSample {
    fn default() -> Self {
        Self { value: None, color: Color::GRAY }
    }
}

impl BarsSample {
    pub fn set_value(&mut self, val: f64) { self.value = Some(val); }
    pub fn hide(&mut self) { self.value = None; }
}

impl Bars {
    pub fn histogram(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            bar_width: BarWidth::Thin,
            baseline: Baseline::Zero,
            offset: 0,
            default_sample: BarsSample::default(),
        }
    }

    pub fn columns(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            bar_width: BarWidth::Wide,
            baseline: Baseline::Bottom,
            offset: 0,
            default_sample: BarsSample::default(),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.default_sample.color = color; self }
    pub fn set_offset(mut self, offset: i32) -> Self { self.offset = offset; self }
    pub fn set_baseline(mut self, baseline: Baseline) -> Self { self.baseline = baseline; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Bars {
    type Sample = BarsSample;
    fn new_sample(&self) -> BarsSample { self.default_sample }
}


// ============
// === Fill ===
// ============

#[derive(Clone, Copy, Debug, Default)]
pub struct Fill {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub offset: i32,
    /// Whether to continue filling across gaps (`None` values).
    pub fill_gaps: bool,
    pub default_sample: FillSample,
}

/// Color interpolation between two price levels within a fill region.
#[derive(Clone, Copy, Debug)]
pub struct FillGradient {
    pub top_value: f64,
    pub bottom_value: f64,
    pub top_color: Color,
    pub bottom_color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct FillSample {
    pub value_a: Option<f64>,
    /// `Some`=fill between two values. `None`=fill from `value_a` to pane bottom.
    pub value_b: Option<f64>,
    pub color: Color,
    /// When set,overrides `color` with a gradient between two price levels.
    pub gradient: Option<FillGradient>,
}

impl Default for FillSample {
    fn default() -> Self {
        Self { value_a: None, value_b: None, color: Color::rgba(0, 0, 255, 0.1), gradient: None }
    }
}

impl FillSample {
    /// Fill between two values (e.g. Bollinger upper and lower).
    pub fn set_values(&mut self, val_a: f64, val_b: f64) {
        self.value_a = Some(val_a);
        self.value_b = Some(val_b);
    }

    /// Fill from a value to the bottom of the pane (area chart).
    pub fn set_value(&mut self, val: f64) {
        self.value_a = Some(val);
        self.value_b = None;
    }

    pub fn set_gradient(&mut self, gradient: FillGradient) {
        self.gradient = Some(gradient);
    }

    pub fn hide(&mut self) {
        self.value_a = None;
        self.value_b = None;
        self.gradient = None;
    }
}

impl Fill {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            offset: 0,
            fill_gaps: false,
            default_sample: FillSample::default(),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.default_sample.color = color; self }
    pub fn set_offset(mut self, offset: i32) -> Self { self.offset = offset; self }
    pub fn set_fill_gaps(mut self, fill_gaps: bool) -> Self { self.fill_gaps = fill_gaps; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Fill {
    type Sample = FillSample;
    fn new_sample(&self) -> FillSample { self.default_sample }
}


// ==============
// === Candle ===
// ==============

#[derive(Clone, Copy, Debug, Default)]
pub struct Candle {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    pub style: CandleStyle,
    pub bullish_color: Color,
    pub bearish_color: Color,
    pub wick_color: Color,
    pub border_color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct CandleSample {
    pub ohlc: Option<Ohlc>,
    pub color: Color,
    pub wick_color: Color,
    pub border_color: Color,
}

impl Default for CandleSample {
    fn default() -> Self {
        Self {
            ohlc: None,
            color: Color::GREEN,
            wick_color: Color::GRAY,
            border_color: Color::GRAY,
        }
    }
}

impl CandleSample {
    pub fn set_ohlc(&mut self, open: f64, high: f64, low: f64, close: f64) {
        self.ohlc = Some(Ohlc { open, high, low, close });
    }

    pub fn hide(&mut self) { self.ohlc = None; }
}

impl Candle {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            style: CandleStyle::Candlestick,
            bullish_color: Color::GREEN,
            bearish_color: Color::RED,
            wick_color: Color::GRAY,
            border_color: Color::GRAY,
        }
    }

    pub fn set_bullish_color(mut self, color: Color) -> Self { self.bullish_color = color; self }
    pub fn set_bearish_color(mut self, color: Color) -> Self { self.bearish_color = color; self }
    pub fn set_wick_color(mut self, color: Color) -> Self { self.wick_color = color; self }
    pub fn set_border_color(mut self, color: Color) -> Self { self.border_color = color; self }
    pub fn set_style(mut self, style: CandleStyle) -> Self { self.style = style; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }

    pub fn new_ohlc_sample(&self, open: f64, high: f64, low: f64, close: f64) -> CandleSample {
        let bullish = close >= open;
        CandleSample {
            ohlc: Some(Ohlc { open, high, low, close }),
            color: if bullish { self.bullish_color } else { self.bearish_color },
            wick_color: self.wick_color,
            border_color: self.border_color,
        }
    }
}

impl plot::Component for Candle {
    type Sample = CandleSample;
    fn new_sample(&self) -> CandleSample {
        CandleSample {
            ohlc: None,
            color: self.bullish_color,
            wick_color: self.wick_color,
            border_color: self.border_color,
        }
    }
}


// =============
// === Shape ===
// =============

#[derive(Clone, Copy, Debug, Default)]
pub struct Shape {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub icon: ShapeIcon,
    pub location: ShapeLocation,
    pub size: TextSize,
    pub default_sample: ShapeSample,
}

#[derive(Clone, Copy, Debug)]
pub struct ShapeSample {
    pub visible: bool,
    /// Price position for `ShapeLocation::Absolute`.
    pub price: Option<f64>,
    pub color: Color,
    pub text_color: Color,
    /// Per-bar text displayed near the shape. Supports different static strings per bar
    /// (e.g. "Buy" vs "Sell"). For truly dynamic text with embedded runtime values,
    /// a string pool is needed (not yet implemented).
    pub text: Option<&'static str>,
}

impl Default for ShapeSample {
    fn default() -> Self {
        Self { visible: false, price: None, color: Color::BLUE, text_color: Color::BLUE, text: None }
    }
}

impl ShapeSample {
    pub fn show(&mut self) { self.visible = true; }
    pub fn show_at(&mut self, price: f64) { self.visible = true; self.price = Some(price); }
    pub fn hide(&mut self) { self.visible = false; self.price = None; }
}

impl Shape {
    pub fn new(label: &'static str, icon: ShapeIcon) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            icon,
            location: ShapeLocation::AboveBar,
            size: TextSize::Normal,
            default_sample: ShapeSample::default(),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.default_sample.color = color; self }
    pub fn set_text_color(mut self, color: Color) -> Self { self.default_sample.text_color = color; self }
    pub fn set_text(mut self, text: &'static str) -> Self { self.default_sample.text = Some(text); self }
    pub fn set_location(mut self, location: ShapeLocation) -> Self { self.location = location; self }
    pub fn set_size(mut self, size: TextSize) -> Self { self.size = size; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Shape {
    type Sample = ShapeSample;
    fn new_sample(&self) -> ShapeSample { self.default_sample }
}


// =============
// === Arrow ===
// =============

#[derive(Clone, Copy, Debug, Default)]
pub struct Arrow {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    /// Minimum arrow height in pixels. Even the smallest non-zero value produces
    /// an arrow at least this tall.
    pub min_pixel_height: u32,
    /// Maximum arrow height in pixels. The largest value in the visible range
    /// maps to this height. Arrow height is linearly interpolated between
    /// `min_pixel_height` and `max_pixel_height` based on the absolute value.
    pub max_pixel_height: u32,
    pub default_sample: ArrowSample,
}

#[derive(Clone, Copy, Debug)]
pub struct ArrowSample {
    /// Positive=up arrow,negative=down arrow,None=hidden.
    /// Arrow length proportional to absolute value.
    pub value: Option<f64>,
    pub color_up: Color,
    pub color_down: Color,
}

impl Default for ArrowSample {
    fn default() -> Self {
        Self { value: None, color_up: Color::GREEN, color_down: Color::RED }
    }
}

impl ArrowSample {
    pub fn set_value(&mut self, val: f64) { self.value = Some(val); }
    pub fn hide(&mut self) { self.value = None; }
}

impl Arrow {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            min_pixel_height: 5,
            max_pixel_height: 100,
            default_sample: ArrowSample::default(),
        }
    }

    pub fn set_color_up(mut self, color: Color) -> Self { self.default_sample.color_up = color; self }
    pub fn set_color_down(mut self, color: Color) -> Self { self.default_sample.color_down = color; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for Arrow {
    type Sample = ArrowSample;
    fn new_sample(&self) -> ArrowSample { self.default_sample }
}


// =============
// === HLine ===
// =============

#[derive(Clone, Copy, Debug, Default)]
pub struct HLine {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub format: ValueFormat,
    pub precision: Option<u8>,
    pub price: f64,
    pub color: Color,
    pub style: LineStyle,
    pub width: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HLineSample;

impl HLine {
    pub fn new(label: &'static str, price: f64) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            format: ValueFormat::Inherit,
            precision: None,
            price,
            color: Color::GRAY,
            style: LineStyle::Dashed,
            width: 1,
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.color = color; self }
    pub fn set_style(mut self, style: LineStyle) -> Self { self.style = style; self }
    pub fn set_width(mut self, width: u8) -> Self { self.width = width; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for HLine {
    type Sample = HLineSample;
    fn new_sample(&self) -> HLineSample { HLineSample }
}


// =================
// === HLineFill ===
// =================

#[derive(Clone, Copy, Debug, Default)]
pub struct HLineFill {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
    pub price_a: f64,
    pub price_b: f64,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HLineFillSample;

impl HLineFill {
    pub fn between(label: &'static str, hline_a: &HLine, hline_b: &HLine) -> Self {
        Self {
            label,
            display: Display::ALL,
            show_last: None,
            overlay: false,
            price_a: hline_a.price,
            price_b: hline_b.price,
            color: Color::rgba(128, 128, 128, 0.05),
        }
    }

    pub fn set_color(mut self, color: Color) -> Self { self.color = color; self }
    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for HLineFill {
    type Sample = HLineFillSample;
    fn new_sample(&self) -> HLineFillSample { HLineFillSample }
}


// ===============
// === BgColor ===
// ===============

#[derive(Clone, Copy, Debug, Default)]
pub struct BgColor {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
    pub overlay: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BgColorSample {
    pub color: Option<Color>,
}

impl BgColorSample {
    pub fn set(&mut self, color: Color) { self.color = Some(color); }
    pub fn clear(&mut self) { self.color = None; }
}

impl BgColor {
    pub fn new(label: &'static str) -> Self {
        Self { label, display: Display::ALL, show_last: None, overlay: false }
    }

    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
    pub fn set_overlay(mut self, overlay: bool) -> Self { self.overlay = overlay; self }
}

impl plot::Component for BgColor {
    type Sample = BgColorSample;
    fn new_sample(&self) -> BgColorSample { BgColorSample::default() }
}


// ================
// === BarColor ===
// ================

#[derive(Clone, Copy, Debug, Default)]
pub struct BarColor {
    pub label: &'static str,
    pub display: Display,
    pub show_last: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BarColorSample {
    pub color: Option<Color>,
}

impl BarColorSample {
    pub fn set(&mut self, color: Color) { self.color = Some(color); }
    pub fn clear(&mut self) { self.color = None; }
}

impl BarColor {
    pub fn new(label: &'static str) -> Self {
        Self { label, display: Display::ALL, show_last: None }
    }

    pub fn set_display(mut self, display: Display) -> Self { self.display = display; self }
    pub fn set_show_last(mut self, n: usize) -> Self { self.show_last = Some(n); self }
}

impl plot::Component for BarColor {
    type Sample = BarColorSample;
    fn new_sample(&self) -> BarColorSample { BarColorSample::default() }
}


// ==============
// === Stream ===
// ==============

#[derive(Deref, DerefMut)]
pub struct Stream<T: Indicator> {
    pub state: State<T>,
}

impl<T: Indicator> Debug for Stream<T> where
State<T>: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.state, f)
    }
}

impl<T: Indicator> Stream<T> {
    pub fn new(indicator: T) -> Self {
        let state = T::State::new(indicator);
        Self { state }
    }
}


// ================================
// === ExponentialMovingAverage ===
// ================================

pub mod exponential_moving_average {
    use super::*;


    // =================
    // === Indicator ===
    // =================

    // #[derive(Indicator)]
    // #[indicator(short_label="EMA",categories=[Trend])]
    // #[plot(overlay=true)]
    #[derive(Clone, Copy, Debug)]
    pub struct ExponentialMovingAverage {
        pub period: usize,
    }

    pub type Input = f64;
    pub type Output = f64;


    // =============
    // === State ===
    // =============

    #[derive(Clone, Copy, Debug)]
    pub struct State {
        multiplier: f64,
        current: Option<f64>,
    }

    impl IndicatorState for State {
        fn new(config: ExponentialMovingAverage) -> Self {
            Self {
                multiplier: 2.0 / (config.period as f64 + 1.0),
                current: None,
            }
        }

        #[inline(always)]
        fn push_value(&mut self, input: f64) -> StepResult<ExponentialMovingAverage> {
            let result = match self.current {
                None => input,
                Some(previous) => input * self.multiplier + previous * (1.0 - self.multiplier),
            };
            self.current = Some(result);
            StepResult { output: result, alerts: NoAlerts }
        }
    }


    // ============
    // === Plot ===
    // ============

    // #[derive(IndicatorPlot)]
    #[derive(Clone, Copy, Debug)]
    pub struct Plot {
        line: Line,
    }

    impl IndicatorPlot for Plot {
        fn new() -> Self {
            Self {
                line: Line::new("EMA").set_color(Color::BLUE).set_width(2),
            }
        }

        fn sample(&self, ctx: &PlotContext<'_, f64>) -> plot::Sample<Self> {
            let mut out = self.new_sample();
            out.line.set_value(ctx.current_output());
            out
        }
    }


    // === To be generated by macros ===

    impl Default for ExponentialMovingAverage {
        fn default() -> Self {
            Self { period: default() }
        }
    }

    impl IndicatorAssoc for State {
        type Indicator = ExponentialMovingAverage;
    }

    impl Indicator for ExponentialMovingAverage {
        type Input = f64;
        type Output = f64;
        type Alerts = NoAlerts;
        type State = State;
        type Plot = Plot;

        fn categories() -> Categories {
            &[Category::Trend]
        }

        fn labels() -> Labels {
            Labels {
                full: "Exponential Moving Average",
                short: "EMA",
                aliases: &[],
            }
        }

        fn plot_config() -> plot::Config {
            plot::Config {
                overlay: true,
                ..Default::default()
            }
        }
    }

    impl IndicatorAssoc for Plot {
        type Indicator = ExponentialMovingAverage;
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct PlotSample {
        line: plot::Sample<Line>,
    }

    impl plot::Component for Plot {
        type Sample = PlotSample;

        fn new_sample(&self) -> PlotSample {
            PlotSample {
                line: self.line.new_sample(),
            }
        }
    }
}

use exponential_moving_average::ExponentialMovingAverage;


// ==========================================
// === MovingAverageConvergenceDivergence ===
// ==========================================

pub mod moving_average_convergence_divergence {
    use super::*;


    // =================
    // === Indicator ===
    // =================

    // #[derive(Indicator)]
    // #[indicator(short_label="MACD",categories=[Momentum,Trend])]
    // #[plot(value_domain=Unbounded)]
    #[derive(Clone, Copy, Debug)]
    pub struct MovingAverageConvergenceDivergence {
        // #[param(min=1,recommended_min=5,recommended_max=20)]
        pub fast_period: usize,
        // #[param(min=1,recommended_min=15,recommended_max=50)]
        pub slow_period: usize,
        // #[param(min=1,recommended_min=3,recommended_max=20)]
        pub signal_period: usize,
    }

    pub type Input = f64;

    #[derive(Clone, Copy, Debug)]
    pub struct Output {
        pub macd: f64,
        pub signal: f64,
        pub hist: f64,
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Alerts {
        pub bullish_cross: bool,
        pub bearish_cross: bool,
    }


    // =============
    // === State ===
    // =============

    #[derive(Debug)]
    pub struct State {
        fast_ema: Stream<ExponentialMovingAverage>,
        slow_ema: Stream<ExponentialMovingAverage>,
        signal_ema: Stream<ExponentialMovingAverage>,
        prev_macd: f64,
        prev_signal: f64,
    }

    impl IndicatorState for State {
        fn new(config: MovingAverageConvergenceDivergence) -> Self {
            Self {
                fast_ema: Stream::new(ExponentialMovingAverage { period: config.fast_period }),
                slow_ema: Stream::new(ExponentialMovingAverage { period: config.slow_period }),
                signal_ema: Stream::new(ExponentialMovingAverage { period: config.signal_period }),
                prev_macd: 0.0,
                prev_signal: 0.0,
            }
        }

        #[inline(always)]
        fn push_value(&mut self, input: Input) -> StepResult<MovingAverageConvergenceDivergence> {
            let fast = self.fast_ema.push_value(input).output;
            let slow = self.slow_ema.push_value(input).output;
            let macd = fast - slow;
            let signal = self.signal_ema.push_value(macd).output;
            let hist = macd - signal;

            let alerts = Alerts {
                bullish_cross: self.prev_macd <= self.prev_signal && macd > signal,
                bearish_cross: self.prev_macd >= self.prev_signal && macd < signal,
            };
            self.prev_macd = macd;
            self.prev_signal = signal;

            StepResult {
                output: Output { macd, signal, hist },
                alerts,
            }
        }
    }


    // ============
    // === Plot ===
    // ============

    // #[derive(plot::Component)]
    #[derive(Clone, Copy, Debug)]
    pub struct Plot {
        macd_line: Line,
        signal_line: Line,
        histogram: Bars,
    }

    impl IndicatorPlot for Plot {
        fn new() -> Self {
            Self {
                macd_line: Line::new("MACD").set_color(Color::BLUE),
                signal_line: Line::new("Signal").set_color(Color::ORANGE),
                histogram: Bars::histogram("Histogram"),
            }
        }

        fn sample(&self, ctx: &PlotContext<'_, Output>) -> plot::Sample<Self> {
            let mut out = self.new_sample();
            let current_output = ctx.current_output();

            out.macd_line.set_value(current_output.macd);
            out.signal_line.set_value(current_output.signal);
            out.histogram.set_value(current_output.hist);

            let prev_hist = ctx.prev_output(1).map_or(0.0, |prev| prev.hist);
            let rising = current_output.hist >= prev_hist;
            out.histogram.color = match (current_output.hist >= 0.0, rising) {
                (true, true) => Color::rgb(38, 166, 91),
                (true, false) => Color::rgb(147, 210, 174),
                (false, false) => Color::rgb(239, 67, 82),
                (false, true) => Color::rgb(246, 167, 172),
            };

            out
        }
    }


    // === To be generated by macros ===

    impl Default for MovingAverageConvergenceDivergence {
        fn default() -> Self {
            Self {
                fast_period: 5,
                slow_period: 15,
                signal_period: 3,
            }
        }
    }

    impl IndicatorAssoc for State {
        type Indicator = MovingAverageConvergenceDivergence;
    }

    impl Indicator for MovingAverageConvergenceDivergence {
        type Input = f64;
        type Output = Output;
        type Alerts = Alerts;
        type State = State;
        type Plot = Plot;

        fn categories() -> Categories {
            &[Category::Momentum, Category::Trend]
        }

        fn labels() -> Labels {
            Labels {
                full: "Moving Average Convergence Divergence",
                short: "MACD",
                aliases: &[],
            }
        }

        fn alert_signals() -> &'static [AlertSignal] {
            &[
            AlertSignal {
                label: "Bullish Cross",
                description: "MACD line crosses above signal line",
                enabled_by_default: true,
            },
            AlertSignal {
                label: "Bearish Cross",
                description: "MACD line crosses below signal line",
                enabled_by_default: true,
            },
            ]
        }

        fn plot_config() -> plot::Config {
            plot::Config {
                value_domain: ValueDomain::Unbounded,
                ..Default::default()
            }
        }
    }

    impl IndicatorAssoc for Plot {
        type Indicator = MovingAverageConvergenceDivergence;
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct PlotSample {
        macd_line: plot::Sample<Line>,
        signal_line: plot::Sample<Line>,
        histogram: plot::Sample<Bars>,
    }

    impl plot::Component for Plot {
        type Sample = PlotSample;

        fn new_sample(&self) -> PlotSample {
            PlotSample {
                macd_line: self.macd_line.new_sample(),
                signal_line: self.signal_line.new_sample(),
                histogram: self.histogram.new_sample(),
            }
        }
    }
}
