pub mod glass_pipeline;

pub use glass_pipeline::{GlassPipeline, GlassUniforms, Vertex};

/// Helper to generate a default Frutiger Aero aesthetic procedural background (Aqua/Blue sky and glass orb gradient)
pub fn generate_frutiger_aero_gradient(width: u32, height: u32) -> Vec<u8> {
    let mut buffer = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let v = y as f32 / height as f32; // 0.0 top to 1.0 bottom
        for x in 0..width {
            let u = x as f32 / width as f32;

            // Frutiger Aero signature gradient: Cyan/Sky Blue -> Bright Emerald/Aqua -> Deep Navy Azure
            let r = ((0.15 + 0.35 * (1.0 - v) + 0.1 * (u * std::f32::consts::PI).sin()) * 255.0).clamp(0.0, 255.0) as u8;
            let g = ((0.55 + 0.35 * (1.0 - v) + 0.1 * (1.0 - (u - 0.5).abs())) * 255.0).clamp(0.0, 255.0) as u8;
            let b = ((0.85 + 0.15 * (1.0 - v)) * 255.0).clamp(0.0, 255.0) as u8;
            let a = 255u8;

            buffer.push(r);
            buffer.push(g);
            buffer.push(b);
            buffer.push(a);
        }
    }

    buffer
}
