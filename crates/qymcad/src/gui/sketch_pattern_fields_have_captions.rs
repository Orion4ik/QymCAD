//! SKETCH PATTERN FIELDS ON THE OPTIONS BAR CARRY CAPTIONS.
//!
//! Reported behaviour: the step and angle fields of the sketch patterns stand on the options bar
//! without captions or units.
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::{App, Sel};
    use qymcad_core::feature::SketchPlane;

    #[test]
    fn sketch_linear_pattern_options_bar_labels_step_fields() {
        let mut app = App::default();
        app.set.gpu_viewport = false;
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        let mut hand = Hand::new(&mut app);
        assert!(hand.press_hint(&qymcad_i18n::tr("tb-lin-array-hint")), "no linear pattern button");
        assert!(hand.shows(&qymcad_i18n::tr("opt-dx")), "linear pattern bar must label dx");
        assert!(hand.shows(&qymcad_i18n::tr("opt-dy")), "linear pattern bar must label dy");
    }

    #[test]
    fn sketch_linear_pattern_options_bar_labels_row_step_fields_when_rows_more_than_one() {
        let mut app = App::default();
        app.set.gpu_viewport = false;
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        app.sk_pat.count2 = 2;
        let mut hand = Hand::new(&mut app);
        assert!(hand.press_hint(&qymcad_i18n::tr("tb-lin-array-hint")), "no linear pattern button");
        assert!(hand.shows(&qymcad_i18n::tr("opt-dx2")), "linear pattern bar must label dx2 when rows > 1");
        assert!(hand.shows(&qymcad_i18n::tr("opt-dy2")), "linear pattern bar must label dy2 when rows > 1");
    }

    #[test]
    fn sketch_circular_pattern_options_bar_labels_angle_field() {
        let mut app = App::default();
        app.set.gpu_viewport = false;
        let si = app.create_sketch_on(SketchPlane::default());
        app.chosen.sel = Sel::Sketch(si);
        let mut hand = Hand::new(&mut app);
        assert!(hand.press_hint(&qymcad_i18n::tr("tb-circ-array-hint")), "no circular pattern button");
        assert!(hand.shows(&qymcad_i18n::tr("opt-angle")), "circular pattern bar must label angle");
    }
}
