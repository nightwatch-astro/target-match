//! End-to-end example: identify which catalogued object a frame captured.
//!
//! Run with: `cargo run --example identify`
//!
//! `anyhow` is used here for ergonomic error handling — it is an example/test-only
//! dependency and never appears in the library's public API.

use anyhow::Result;
use skymath::{Angle, Equatorial, ParseMode};
use target_match::{rank, Constraint, Field, Optics, RadiusPolicy, SkyObject};

/// A minimal catalogue entry. `target-match` never reads `name` — only `position`.
struct Target {
    name: &'static str,
    ra_deg: f64,
    dec_deg: f64,
}

impl SkyObject for Target {
    fn position(&self) -> Equatorial {
        // Catalogue coordinates are J2000.
        Equatorial::j2000(
            Angle::from_degrees(self.ra_deg),
            Angle::from_degrees(self.dec_deg),
        )
        .expect("valid catalogue coordinates")
    }
}

fn main() -> Result<()> {
    // A tiny bring-your-own catalogue (in a real app this comes from your resolver/DB).
    let catalog = [
        Target {
            name: "M 31 (Andromeda)",
            ra_deg: 10.6847,
            dec_deg: 41.2688,
        },
        Target {
            name: "M 110",
            ra_deg: 10.0921,
            dec_deg: 41.6853,
        },
        Target {
            name: "M 33 (Triangulum)",
            ra_deg: 23.4621,
            dec_deg: 30.6599,
        },
    ];

    // Where the scope pointed (parsed from sexagesimal), and its optics.
    let pointing = Equatorial::parse_j2000("00:42:44.3", "+41:16:09", ParseMode::Strict)?;
    let field = Field::from_optics(Optics {
        focal_mm: 800.0,
        pixel_um: (3.76, 3.76),
        binning: (1, 1),
        pixels: (6248, 4176),
    })?;

    println!(
        "Frame: {} × {} (pixel scale {:.3}\"/px)",
        fmt_deg(field.width().degrees()),
        fmt_deg(field.height().degrees()),
        field.pixel_scale().map(|(x, _)| x).unwrap_or(f64::NAN),
    );

    // Every catalogued object within the frame's search radius, nearest first.
    // `within` uses a circular approximation (the circumscribed circle); swap in
    // `Constraint::frame(&field)` to test the exact sensor rectangle instead.
    let hits = rank(
        pointing,
        &catalog,
        Constraint::within(&field, RadiusPolicy::Circumscribed).all(),
    );

    println!("Objects on frame ({}):", hits.len());
    for h in &hits {
        println!(
            "  {:<18} sep {:>6.2}′  PA {:>5.1}°",
            h.object.name,
            h.separation.arcminutes(),
            h.position_angle.degrees(),
        );
    }

    match hits.first() {
        Some(best) => println!("\nBest target: {}", best.object.name),
        None => println!("\nNo catalogued target on this frame."),
    }
    Ok(())
}

fn fmt_deg(d: f64) -> String {
    format!("{d:.3}°")
}
