//! Record the editor's actual glyph runs and decoration lines in document coordinates.
use super::{color_to_rgb, stroke_attrs, xml_escape, Color, Rect, SceneNode};
use crate::widgets::{canvas_viewport_text::paint_text_node, PaintCx};
use crate::{Point2D, RenderBackend, TextBaselineRequest, TextLayout};
use std::fmt::Write as _;

pub(super) fn paint(out: &mut String, node: &SceneNode, metrics: &mut dyn RenderBackend) {
    let _ = write!(
        out,
        r#"<g id="{}" xml:space="preserve">"#,
        xml_escape(&node.id)
    );
    let mut recorder = SvgText { out, metrics };
    paint_text_node(
        &mut PaintCx {
            backend: &mut recorder,
        },
        node,
        node.bounds,
        1.,
        &None,
    );
    out.push_str("</g>");
}

struct SvgText<'a> {
    out: &'a mut String,
    metrics: &'a mut dyn RenderBackend,
}

impl RenderBackend for SvgText<'_> {
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        for run in layout.runs() {
            let c = run.color;
            let _ = write!(
                self.out,
                r#"<text x="{}" y="{}" font-family="{}" font-size="{}" font-weight="{}" font-style="{}" fill="rgb({},{},{})" fill-opacity="{}">{}</text>"#,
                origin.x + run.origin.x,
                origin.y + run.origin.y,
                xml_escape(&run.font_family),
                run.font_size,
                run.font_weight,
                if layout.italic() { "italic" } else { "normal" },
                c.r(),
                c.g(),
                c.b(),
                f32::from(c.a()) / 255.,
                xml_escape(&run.content)
            );
        }
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        let _ = write!(
            self.out,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {}/>"#,
            from.x,
            from.y,
            to.x,
            to.y,
            stroke_attrs(color, width)
        );
    }
    fn fill_rect(&mut self, r: Rect, c: Color) {
        let _ = write!(
            self.out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="{}"/>"#,
            r.origin.x,
            r.origin.y,
            r.size.x,
            r.size.y,
            color_to_rgb(c),
            c.a
        );
    }
    fn measure_text_family_styled(
        &mut self,
        text: &str,
        size: f32,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.metrics
            .measure_text_family_styled(text, size, family, weight, italic)
    }
    fn text_ascent_family(&mut self, size: f32, family: &str, weight: u16) -> f32 {
        self.metrics.text_ascent_family(size, family, weight)
    }
    fn text_first_baseline(&mut self, request: &TextBaselineRequest<'_>) -> f32 {
        self.metrics.text_first_baseline(request)
    }
    // Glyphs remain exportable below the zoomed-out viewport's greeking threshold.
    fn dpi_scale(&self) -> f32 {
        f32::MAX
    }
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
}
