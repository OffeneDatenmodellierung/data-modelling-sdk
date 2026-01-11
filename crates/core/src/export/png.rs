//! PNG exporter for generating PNG images from data models.
//!
//! # Limitations
//!
//! This exporter generates a diagram-only PNG image showing table rectangles
//! arranged in a grid layout. **Text rendering (table names, column names) is
//! not currently supported** due to the complexity of font handling.
//!
//! For exports that include text, consider using:
//! - [`JSONSchemaExporter`](super::json_schema::JSONSchemaExporter) for structured data
//! - DrawIO export in the application layer for visual diagrams with text
//! - SVG export (if available) for scalable graphics with text support
//!
//! # Feature Requirements
//!
//! This module requires the `png-export` feature flag to be enabled:
//! ```toml
//! [dependencies]
//! data-modelling-sdk = { version = "...", features = ["png-export"] }
//! ```
//!
//! # Output Format
//!
//! The exported PNG is returned as base64-encoded data in the `ExportResult.content` field.
//! Decode it using standard base64 libraries to obtain raw PNG bytes.

use super::{ExportError, ExportResult};
use crate::models::Table;
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, ImageEncoder, Rgb, RgbImage};

/// Exporter for PNG image format.
///
/// Generates a visual diagram of tables as rectangles arranged in a grid.
///
/// # Limitations
///
/// - **No text rendering**: Table names and column names are not displayed.
///   This is a diagram-only export intended for visual structure overview.
/// - Tables are arranged in a fixed 4-column grid layout.
/// - Relationships between tables are not shown.
///
/// For full-featured visual exports, use DrawIO export in the application layer.
pub struct PNGExporter;

impl PNGExporter {
    /// Create a new PNG exporter instance.
    pub fn new() -> Self {
        Self
    }

    /// Export tables to PNG image format.
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to include in the diagram
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Returns
    ///
    /// An `ExportResult` with base64-encoded PNG data in the `content` field.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let exporter = PNGExporter::new();
    /// let result = exporter.export(&tables, 1200, 800)?;
    /// let png_bytes = base64::decode(&result.content)?;
    /// std::fs::write("diagram.png", png_bytes)?;
    /// ```
    pub fn export(
        &self,
        tables: &[Table],
        width: u32,
        height: u32,
    ) -> Result<ExportResult, ExportError> {
        let png_bytes = Self::export_model_from_tables(tables, width, height)?;
        Ok(ExportResult {
            content: general_purpose::STANDARD.encode(&png_bytes),
            format: "png".to_string(),
        })
    }

    fn export_model_from_tables(
        tables: &[Table],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let tables_to_export: Vec<&Table> = tables.iter().collect();

        // Create image buffer
        let mut img: RgbImage = ImageBuffer::new(width, height);

        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = Rgb([255u8, 255u8, 255u8]);
        }

        // Draw tables as rectangles in a grid layout
        for (i, _table) in tables_to_export.iter().enumerate() {
            let x = (i as u32 % 4) * (width / 4) + 50;
            let y = (i as u32 / 4) * 200 + 50;

            // Draw table rectangle
            let table_width = 200u32;
            let table_height = 150u32;

            // Draw border (black outline)
            for px in x..(x + table_width).min(width) {
                for py in y..(y + table_height).min(height) {
                    if px == x || px == x + table_width - 1 || py == y || py == y + table_height - 1
                    {
                        img.put_pixel(px, py, Rgb([0u8, 0u8, 0u8]));
                    }
                }
            }

            // Note: Text rendering is not implemented.
            // Adding font support would require:
            // 1. Embedding fonts or loading system fonts
            // 2. Using imageproc's draw_text_mut with rusttype
            // 3. This adds ~2MB to binary size
            //
            // For text-based diagrams, use DrawIO or SVG export instead.
        }

        // Convert to PNG bytes
        let mut buffer = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            encoder.write_image(&img.into_raw(), width, height, image::ColorType::Rgb8)?;
        }

        Ok(buffer)
    }
}

impl Default for PNGExporter {
    fn default() -> Self {
        Self::new()
    }
}
