use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;
use witness_web::render;
use witness_web::testkit::{World, recompress, replay};

fn main() {
    let mut w = World::new();
    let s = w.original(1, true);
    let options = w.options();
    let cases: Vec<_> = [
        ("sealed", s.jpeg.clone()),
        ("recompressed", recompress(&s.jpeg, 70)),
    ]
    .into_iter()
    .map(|(name, input)| {
        let (step, rounds, fetched) = replay(&input, &options, &w.oracle);
        json!({
            "name": name,
            "input": STANDARD.encode(&input),
            "fetched": fetched,
            "rounds": rounds,
            "verdict": render::verdict(&step.verdict),
        })
    })
    .collect();
    println!("{}", json!({"options": options, "cases": cases}));
}
