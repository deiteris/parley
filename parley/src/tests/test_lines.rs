// Copyright 2025 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test line layouts, including the vertical size and positioning of the line box.

use std::borrow::Cow;

use peniko::kurbo::Size;

use super::utils::{ColorBrush, TestEnv};
use crate::{
    Affinity, BoundingBox, Brush, Cursor, FontFamily, FontStack, InlineBox, Layout, LineHeight,
    Selection, StyleProperty, test_name,
};

const TEXT: &str = "Some text here. Let's make\n\
        it a bit longer so that\n \
        we have more lines.\n\
        And also some latin text for\n\
        this spot right here.\n\
        This is underline and\n\
        strikethrough text and some\n\
        extra more text\n\
        and some extra more text\nand some extra more text\n\
        and some extra more text\nand some extra more text\n\
        and some extra more text\nand some extra more text";

const LINE_COUNT: f32 = 14.;

/// Returns the integral max advance that these tests are designed around.
fn max_advance(font_size: f32) -> f32 {
    (200. / 16. * font_size).ceil()
}

/// Returns the precise ascent and descent of Roboto.
fn roboto_ascent_descent(font_size: f32) -> (f32, f32) {
    // We calculate it based on these 16px values
    let ascent = 14.84375;
    let descent = 3.90625;
    (ascent / 16. * font_size, descent / 16. * font_size)
}

/// The expected vertical metrics of every line of a layout.
struct Expected {
    ascent: f32,
    descent: f32,
    /// The line height rounded to a whole pixel.
    rounded: f32,
    /// The font's own line height rounded up to a whole pixel.
    font_height: f32,
}

impl Expected {
    /// The baseline's offset below the top of its line: the ascent plus half the leading
    /// of the rounded line height against the font's own.
    fn baseline_offset(&self) -> f32 {
        self.ascent + (self.rounded - self.font_height) * 0.5
    }

    /// The distance from one line's top to the next.
    fn pitch(&self) -> f32 {
        self.rounded + 1.
    }

    /// The height of the layout: one pixel between lines, none after the last.
    fn layout_height(&self) -> f32 {
        LINE_COUNT * self.rounded + (LINE_COUNT - 1.)
    }
}

/// Returns the expected metrics of a Roboto layout.
fn expected_metrics(font_size: f32, line_height_px: f32) -> Expected {
    let (ascent, descent) = roboto_ascent_descent(font_size);
    Expected {
        ascent,
        descent,
        rounded: line_height_px.round(),
        // Roboto has no line gap, so its own line height is its ascent and descent.
        font_height: (ascent + descent).ceil(),
    }
}

/// Returns selection geometry such that every line is covered.
fn get_selections<B: Brush>(layout: &Layout<B>) -> Vec<(BoundingBox, usize)> {
    let selection_parts = [
        ("Some text here.", 4),
        ("a bit longer", 8),
        ("we have more", 5),
        ("also some latin", 9),
        ("this spot", 7),
        ("is under", 6),
        ("ough text", 4),
        ("more", 4),
        ("me ex", 5),
        ("some", 4),
        ("me ex", 5),
        ("some", 4),
        ("me ex", 5),
        ("some", 4),
    ];

    let mut selections = Vec::new();
    let mut idx = 0;
    for (substr, len) in selection_parts {
        let i = idx + TEXT[idx..].find(substr).unwrap();
        let j = i + len;
        idx = j;
        let selection = Selection::new(
            Cursor::from_byte_index(layout, i, Affinity::Downstream),
            Cursor::from_byte_index(layout, j, Affinity::Downstream),
        );
        selections.extend(selection.geometry(layout));
    }
    selections
}

/// Returns the layout.
fn build_layout<A: Into<Option<f32>>>(
    env: &mut TestEnv,
    font_size: f32,
    line_height: f32,
    max_advance: A,
) -> Layout<ColorBrush> {
    let mut builder = env.ranged_builder(TEXT);
    builder.push_default(StyleProperty::FontSize(font_size));
    builder.push_default(LineHeight::Absolute(line_height));

    let underline_style = StyleProperty::Underline(true);
    let strikethrough_style = StyleProperty::Strikethrough(true);

    // Set the underline & strikethrough style
    let pos_u = TEXT.find("underline").unwrap();
    builder.push(underline_style, pos_u..pos_u + "underline".len());
    let pos_s = TEXT.find("strikethrough").unwrap();
    builder.push(strikethrough_style, pos_s..pos_s + "strikethrough".len());

    builder.push_inline_box(InlineBox {
        id: 0,
        index: 40,
        width: 50.0,
        height: 5.0,
    });
    builder.push_inline_box(InlineBox {
        id: 1,
        index: 51,
        width: 50.0,
        height: 3.0,
    });

    let mut layout = builder.build(TEXT);
    layout.break_all_lines(max_advance.into());
    layout
}

/// Returns the test environment, the layout, and the selections.
fn compute(
    test_name: &str,
    font_size: f32,
    line_height_px: f32,
) -> (TestEnv, Layout<ColorBrush>, Vec<(BoundingBox, usize)>) {
    // Use max advance as the target width to ensure consistency in case of early line breaks
    let width = max_advance(font_size);
    // Calculate the total height of the layout's lines.
    let height = expected_metrics(font_size, line_height_px).layout_height();
    // Compute the layout
    let size = Size::new(width as f64, height as f64);
    let mut env = TestEnv::new(test_name, size);
    let layout = build_layout(&mut env, font_size, line_height_px, width);
    let selections = get_selections(&layout);
    (env, layout, selections)
}

/// Assert expectations that are common across tests.
fn assert_common_truths(layout: &Layout<ColorBrush>, expected: &Expected) {
    let layout_height = expected.layout_height();
    assert_eq!(
        layout.height(),
        layout_height,
        "expected layout height {layout_height}"
    );
    let line_count = layout.lines().count();
    for (index, line) in layout.lines().enumerate() {
        let metrics = line.metrics();
        let ascent = expected.ascent;
        let descent = expected.descent;
        assert_eq!(metrics.ascent, ascent, "expected ascent {ascent}");
        assert_eq!(metrics.descent, descent, "expected descent {descent}");
        let line_height = if index + 1 == line_count {
            expected.rounded
        } else {
            expected.pitch()
        };
        assert_eq!(
            metrics.line_height, line_height,
            "expected line height {line_height}"
        );
        let top = index as f32 * expected.pitch();
        let offset = metrics.baseline - top;
        assert!(
            (offset - expected.baseline_offset()).abs() < 1e-4,
            "expected the baseline {} below the line top, got {offset}",
            expected.baseline_offset()
        );
        // The line box holds the rounded line height and every glyph.
        assert_eq!(metrics.min_coord, top.min(metrics.baseline - ascent));
        assert_eq!(
            metrics.max_coord,
            (top + expected.rounded).max(metrics.baseline + descent)
        );
    }
}

/// Test integral line height equal to the font's line height rounded up.
///
/// There is no leading: the baseline sits the ascent below the line top.
#[test]
fn lines_integral_line_height_zero_leading() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 19.0;

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(
        expected.rounded, expected.font_height,
        "expected the font's line height to round up to the line height"
    );

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    for (index, line) in layout.lines().enumerate() {
        let top = index as f32 * expected.pitch();
        assert_eq!(
            line.metrics().baseline - top,
            expected.ascent,
            "expected zero leading"
        );
    }

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test integral line height that gives a negative leading of -1.
///
/// The baseline rises half a pixel, and the line box still holds every glyph.
#[test]
fn lines_integral_line_height_minus_one_leading() {
    // Inputs
    let font_size = 17.0;
    let line_height_px = 19.0;

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(
        expected.rounded - expected.font_height,
        -1.,
        "expected -1 leading"
    );

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    for line in layout.lines() {
        let metrics = line.metrics();
        assert!(
            metrics.max_coord - metrics.min_coord >= expected.ascent + expected.descent,
            "expected the line box to hold the glyphs"
        );
    }

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test integral line height that gives a positive leading of 1.
///
/// Half of it lies above the glyphs, and the line box is the line height.
#[test]
fn lines_integral_line_height_plus_one_leading() {
    // Inputs
    let font_size = 15.0;
    let line_height_px = 19.0;

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(
        expected.rounded - expected.font_height,
        1.,
        "expected +1 leading"
    );

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    for line in layout.lines() {
        let metrics = line.metrics();
        let above = metrics.baseline - metrics.min_coord;
        assert_eq!(
            above,
            expected.ascent + 0.5,
            "expected above to be the ascent and half a pixel"
        );
        assert_eq!(
            metrics.max_coord - metrics.min_coord,
            line_height_px,
            "expected the line box to be the line height"
        );
    }

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test that the font's line height is its ascent and descent summed, then rounded up.
///
/// With Roboto 19.0px the ascent and descent are 17.626953 and 4.638672, which sum
/// to 22.265625: 23 rounded up, where rounding would give 22.
#[test]
fn lines_integral_line_height_ascent_descent_rounding() {
    // Inputs
    let font_size = 19.0;
    let line_height_px = 20.0; // Just something lower than 23.0 (ascent+descent)

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(expected.font_height, 23., "expected the sum rounded up");
    assert_eq!(
        (expected.ascent + expected.descent).round(),
        22.,
        "expected ascent and descent to be such that rounding up matters"
    );

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    let first = layout.lines().next().unwrap();
    assert_eq!(
        first.metrics().baseline,
        expected.ascent - 1.5,
        "expected the baseline 1.5 px above the ascent"
    );

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test handling of line height that rounds up.
#[test]
fn lines_line_height_rounds_up() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 20.7; // Greater than ascent + descent

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(expected.rounded, 21., "expected 20.7 to round up");

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    let first = layout.lines().next().unwrap();
    assert_eq!(
        first.metrics().line_height,
        22.,
        "expected a pitch of the rounded line height and one pixel"
    );

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test handling of line height that rounds down.
#[test]
fn lines_line_height_rounds_down() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 20.3; // Greater than ascent + descent

    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);
    assert_eq!(expected.rounded, 20., "expected 20.3 to round down");

    // Compute
    let (mut env, layout, selections) = compute(test_name!(), font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    let first = layout.lines().next().unwrap();
    assert_eq!(
        first.metrics().line_height,
        21.,
        "expected a pitch of the rounded line height and one pixel"
    );

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test fractional line height with a negative leading.
///
/// The line box must not be reduced below the glyphs by the negative leading.
fn lines_fractional_line_height_negative_leading_internal(
    test_name: &str,
    font_size: f32,
    line_height_px: f32,
) {
    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);

    // Compute
    let (mut env, layout, selections) = compute(test_name, font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    for line in layout.lines() {
        let metrics = line.metrics();
        assert!(
            metrics.leading < 0.,
            "expected negative leading, but got {}",
            metrics.leading
        );
        assert!(
            metrics.max_coord - metrics.min_coord >= expected.ascent + expected.descent,
            "expected the line box to hold the glyphs"
        );
    }

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test fractional line height with a negative leading.
///
/// The line box must not be reduced below the glyphs by the negative leading.
#[test]
fn lines_fractional_line_height_negative_leading() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 12.75;

    // Run the test
    lines_fractional_line_height_negative_leading_internal(test_name!(), font_size, line_height_px);
}

/// Test fractional line height with a big negative leading.
///
/// The line box must not be reduced below the glyphs by the negative leading.
#[test]
fn lines_fractional_line_height_big_negative_leading() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 10.8;

    // Run the test
    lines_fractional_line_height_negative_leading_internal(test_name!(), font_size, line_height_px);
}

/// Test fractional line height with a positive leading.
///
/// Half the leading of the rounded line height lies above the glyphs; the rest of
/// the line box, with the rounding of the font's line height, lies below them.
fn lines_fractional_line_height_positive_leading_internal(
    test_name: &str,
    font_size: f32,
    line_height_px: f32,
) {
    // Expected outputs
    let expected = expected_metrics(font_size, line_height_px);

    // Compute
    let (mut env, layout, selections) = compute(test_name, font_size, line_height_px);

    // Confirm metrics
    assert_common_truths(&layout, &expected);
    for line in layout.lines() {
        let metrics = line.metrics();
        assert!(
            metrics.leading > 0.,
            "expected positive leading, but got {}",
            metrics.leading
        );

        let above_leading = metrics.baseline - metrics.min_coord - expected.ascent;
        let below_leading = metrics.max_coord - metrics.baseline - expected.descent;
        let half_leading = (expected.rounded - expected.font_height) * 0.5;
        assert!(
            (above_leading - half_leading).abs() < 1e-4,
            "expected above leading {above_leading} to be half the leading {half_leading}"
        );
        assert!(
            above_leading <= below_leading,
            "expected above leading {above_leading} to be at most below leading {below_leading}"
        );
    }

    // Verify visuals
    env.render_and_check_snapshot(&layout, None, &selections);
}

/// Test fractional line height with a positive leading.
///
/// The line box height must be increased by the leading,
/// divided correctly between the sides of the baseline.
#[test]
fn lines_fractional_line_height_positive_leading() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 24.1;

    // Run the test
    lines_fractional_line_height_positive_leading_internal(test_name!(), font_size, line_height_px);
}

/// Test fractional line height with a big positive leading.
///
/// The line box height must be increased by the leading,
/// divided correctly between the sides of the baseline.
#[test]
fn lines_fractional_line_height_big_positive_leading() {
    // Inputs
    let font_size = 16.0;
    let line_height_px = 46.66;

    lines_fractional_line_height_positive_leading_internal(test_name!(), font_size, line_height_px);
}

#[test]
fn lines_line_height_metrics_relative() {
    let mut env = TestEnv::new(test_name!(), None);

    let mut builder = env.ranged_builder(TEXT);
    builder.push_default(LineHeight::MetricsRelative(1.1));

    let mut layout = builder.build(TEXT);

    layout.break_all_lines(None);
    layout.align(
        None,
        crate::Alignment::Start,
        crate::AlignmentOptions::default(),
    );
    env.check_layout_snapshot(&layout);
}

#[test]
fn lines_line_height_size_relative() {
    let mut env = TestEnv::new(test_name!(), None);

    let mut builder = env.ranged_builder(TEXT);
    builder.push_default(LineHeight::FontSizeRelative(1.2));

    let mut layout = builder.build(TEXT);

    layout.break_all_lines(None);
    layout.align(
        None,
        crate::Alignment::Start,
        crate::AlignmentOptions::default(),
    );
    env.check_layout_snapshot(&layout);
}

#[test]
fn lines_line_height_absolute() {
    let mut env = TestEnv::new(test_name!(), None);

    let mut builder = env.ranged_builder(TEXT);
    builder.push_default(LineHeight::Absolute(20.0));

    let mut layout = builder.build(TEXT);

    layout.break_all_lines(None);
    layout.align(
        None,
        crate::Alignment::Start,
        crate::AlignmentOptions::default(),
    );
    env.check_layout_snapshot(&layout);
}

const FIVE_LINES: &str = "one\ntwo\nthree\nfour\nfive";

/// Returns an unwrapped Arimo layout with an absolute line height.
fn arimo_layout(
    env: &mut TestEnv,
    text: &str,
    font_size: f32,
    line_height: f32,
) -> Layout<ColorBrush> {
    let mut builder = env.ranged_builder(text);
    builder.push_default(StyleProperty::FontStack(FontStack::Single(
        FontFamily::Named(Cow::Borrowed("Arimo")),
    )));
    builder.push_default(StyleProperty::FontSize(font_size));
    builder.push_default(LineHeight::Absolute(line_height));
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout
}

/// Returns the baselines of the layout's lines.
fn baselines(layout: &Layout<ColorBrush>) -> Vec<f32> {
    layout.lines().map(|line| line.metrics().baseline).collect()
}

/// A line height that is not a whole pixel count steps a constant pitch: the height
/// rounded to a whole pixel, plus one pixel. No remainder carries from line to line.
#[test]
fn lines_fractional_line_height_steps_a_constant_rounded_pitch() {
    let mut env = TestEnv::new(test_name!(), None);
    // 17.33 px rounds to 17, so every line steps 18 px.
    let layout = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 52. / 3.);

    let pitches: Vec<f32> = baselines(&layout).windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(pitches, [18.; 4], "expected a constant pitch of 18 px");
}

/// A whole pixel line height also gains the one pixel between lines.
#[test]
fn lines_integral_line_height_steps_one_pixel_more() {
    let mut env = TestEnv::new(test_name!(), None);
    let layout = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 17.);

    let pitches: Vec<f32> = baselines(&layout).windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(pitches, [18.; 4], "expected a constant pitch of 18 px");
}

/// A half pixel line height rounds to even before the pitch pixel is added.
#[test]
fn lines_half_pixel_line_height_rounds_to_even() {
    let mut env = TestEnv::new(test_name!(), None);
    let layout = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 14.5);

    let pitches: Vec<f32> = baselines(&layout).windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(
        pitches, [15.; 4],
        "expected 14.5 px to round to 14, a pitch of 15 px"
    );
}

/// The pixel between lines is not added after the last line:
/// a block of n lines is n times the rounded height plus n - 1 pixels.
#[test]
fn lines_block_height_has_no_pitch_pixel_after_the_last_line() {
    let mut env = TestEnv::new(test_name!(), None);
    let layout = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 52. / 3.);

    assert_eq!(layout.height(), 5. * 17. + 4., "expected 89 px");
    let last = layout.lines().last().unwrap();
    assert_eq!(
        last.metrics().line_height,
        17.,
        "expected the last line to be 17 px"
    );
}

/// The first baseline lies the ascent plus half the leading below the top, where the
/// leading is taken on the rounded line height, against the font's ascent, descent and
/// line gap rounded up to a whole pixel. The pitch pixel lies wholly below the line.
#[test]
fn lines_first_baseline_takes_the_half_leading_of_the_rounded_height() {
    let mut env = TestEnv::new(test_name!(), None);
    let layout = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 52. / 3.);

    let line = layout.lines().next().unwrap();
    let run_metrics = *line.runs().next().unwrap().metrics();
    assert!(
        run_metrics.leading > 0.,
        "expected Arimo to have a line gap"
    );
    let normal_height = (run_metrics.ascent + run_metrics.descent + run_metrics.leading).ceil();
    let expected = run_metrics.ascent + (17. - normal_height) / 2.;
    assert_eq!(line.metrics().baseline, expected);
}

/// A line whose height an inline box sets keeps the box's height,
/// and the text line after it steps from its bottom.
#[test]
fn lines_inline_box_line_keeps_its_height() {
    let mut env = TestEnv::new(test_name!(), None);
    let text = "box\ntext";
    let mut builder = env.ranged_builder(text);
    builder.push_default(StyleProperty::FontStack(FontStack::Single(
        FontFamily::Named(Cow::Borrowed("Arimo")),
    )));
    builder.push_default(StyleProperty::FontSize(40. / 3.));
    builder.push_default(LineHeight::Absolute(52. / 3.));
    builder.push_inline_box(InlineBox {
        id: 0,
        index: 0,
        width: 40.0,
        height: 40.0,
    });
    let mut layout = builder.build(text);
    layout.break_all_lines(None);

    let heights: Vec<f32> = layout
        .lines()
        .map(|line| line.metrics().line_height)
        .collect();
    assert_eq!(
        heights,
        [40., 17.],
        "expected the box line 40 px, the text line 17 px"
    );
    assert_eq!(layout.height(), 57., "expected 40 + 17 px");
}

/// A line whose inline box is no taller than the text's ascent keeps the text's pitch.
#[test]
fn lines_inline_box_as_tall_as_the_ascent_keeps_the_pitch() {
    let mut env = TestEnv::new(test_name!(), None);
    let ascent = arimo_layout(&mut env, FIVE_LINES, 40. / 3., 52. / 3.)
        .lines()
        .next()
        .unwrap()
        .metrics()
        .ascent;
    let mut builder = env.ranged_builder(FIVE_LINES);
    builder.push_default(StyleProperty::FontStack(FontStack::Single(
        FontFamily::Named(Cow::Borrowed("Arimo")),
    )));
    builder.push_default(StyleProperty::FontSize(40. / 3.));
    builder.push_default(LineHeight::Absolute(52. / 3.));
    builder.push_inline_box(InlineBox {
        id: 0,
        index: 0,
        width: 10.0,
        height: ascent,
    });
    let mut layout = builder.build(FIVE_LINES);
    layout.break_all_lines(None);

    let pitches: Vec<f32> = baselines(&layout).windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(pitches, [18.; 4], "expected the box line to step 18 px too");
    assert_eq!(layout.height(), 5. * 17. + 4., "expected 89 px");
}
