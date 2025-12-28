//! PNG exporter for generating PNG images from data models.

use crate::models::{DataModel, Table};
use image::{ImageBuffer, ImageEncoder, Rgb, RgbImage};
use super::{ExportError, ExportResult};
use base64::{Engine as _, engine::general_purpose};

/// Exporter for PNG image format.
pub struct PNGExporter;

impl PNGExporter {
    /// Export tables to PNG image format (SDK interface).
    pub fn export(&self, tables: &[Table], width: u32, height: u32) -> Result<ExportResult, ExportError> {
        let png_bytes = Self::export_model_from_tables(tables, width, height)?;
        Ok(ExportResult {
            content: general_purpose::STANDARD.encode(&png_bytes),
            format: "png".to_string(),
        })
    }

    fn export_model_from_tables(tables: &[Table], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let tables_to_export: Vec<&Table> = tables.iter().collect();

        // Create image buffer
        let mut img: RgbImage = ImageBuffer::new(width, height);

        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = Rgb([255u8, 255u8, 255u8]);
        }

        // Draw tables as rectangles
        for (i, _table) in tables_to_export.iter().enumerate() {
            let x = (i as u32 % 4) * (width / 4) + 50;
            let y = (i as u32 / 4) * 200 + 50;

            // Draw table rectangle
            let table_width = 200u32;
            let table_height = 150u32;

            // Draw border
            for px in x..(x + table_width).min(width) {
                for py in y..(y + table_height).min(height) {
                    if px == x || px == x + table_width - 1 || py == y || py == y + table_height - 1
                    {
                        img.put_pixel(px, py, Rgb([0u8, 0u8, 0u8]));
                    }
                }
            }

            // Draw table name (simplified - would need font loading for proper text rendering)
            // For now, just draw a placeholder rectangle for text area
        }

        // Convert to PNG bytes
        let mut buffer = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            // Use write_image instead of deprecated encode method
            encoder.write_image(&img.into_raw(), width, height, image::ColorType::Rgb8)?;
        }

        Ok(buffer)
    }
}
