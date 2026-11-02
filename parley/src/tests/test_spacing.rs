// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The advance of a space.

use std::borrow::Cow;

use crate::{FontFamily, FontStack, StyleProperty, test_name};

use super::utils::TestEnv;

/// Returns the advance of each cluster of an unwrapped Arimo layout of `text`, at 20 px
/// with `word_spacing`, after checking that each cluster's glyphs advance as far.
fn cluster_advances(env: &mut TestEnv, text: &str, word_spacing: f32) -> Vec<f32> {
    let mut builder = env.ranged_builder(text);
    builder.push_default(StyleProperty::FontStack(FontStack::Single(
        FontFamily::Named(Cow::Borrowed("Arimo")),
    )));
    builder.push_default(StyleProperty::FontSize(20.));
    builder.push_default(StyleProperty::WordSpacing(word_spacing));
    let mut layout = builder.build(text);
    layout.break_all_lines(None);

    let mut advances = Vec::new();
    for line in layout.lines() {
        for item in line.items() {
            if let crate::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                for cluster in glyph_run.run().clusters() {
                    let glyphs: f32 = cluster.glyphs().map(|glyph| glyph.advance).sum();
                    assert_eq!(glyphs, cluster.advance(), "glyphs advance as the cluster");
                    advances.push(cluster.advance());
                }
            }
        }
    }
    advances
}

/// A space advances as far as the `i` of its font, not by the font's own space.
#[test]
fn spacing_a_space_is_as_wide_as_the_i_of_its_font() {
    let mut env = TestEnv::new(test_name!(), None);
    let i = cluster_advances(&mut env, "i", 0.)[0];

    let advances = cluster_advances(&mut env, "xx xx", 0.);

    assert_eq!(advances[2], i, "expected the space to advance as `i`");
}

/// Word spacing is added to the advance of `i`.
#[test]
fn spacing_word_spacing_adds_to_the_i_advance() {
    let mut env = TestEnv::new(test_name!(), None);
    let i = cluster_advances(&mut env, "i", 0.)[0];

    let advances = cluster_advances(&mut env, "xx xx", 5.);

    assert_eq!(advances[2], i + 5., "expected `i` plus the word spacing");
}
