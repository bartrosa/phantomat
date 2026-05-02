//! Arrow [`RecordBatch`] validation for scatter columns (`x`, `y`, `r`, `g`, `b`, `a`, `size`).

use arrow::array::Float32Array;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;

/// Schema / layout errors for [`super::ScatterLayer::from_arrow`](crate::ScatterLayer::from_arrow).
#[derive(Debug, thiserror::Error)]
pub enum ArrowSchemaError {
    #[error("missing column `{0}`")]
    MissingColumn(String),
    #[error("column `{name}` must be Float32, got {dt:?}")]
    WrongType { name: String, dt: DataType },
    #[error("column `{name}` expected length {expected}, got {actual}")]
    WrongLen {
        name: String,
        expected: usize,
        actual: usize,
    },
}

/// Required Float32 column names (same row count `N`).
pub const SCATTER_ARROW_COLS: [&str; 7] = ["x", "y", "r", "g", "b", "a", "size"];

pub fn validate_scatter_batch(batch: &RecordBatch) -> Result<(), ArrowSchemaError> {
    let n = batch.num_rows();
    for name in SCATTER_ARROW_COLS {
        let col = batch.column_by_name(name).ok_or_else(|| {
            ArrowSchemaError::MissingColumn(name.to_string())
        })?;
        let dt = col.data_type().clone();
        if dt != DataType::Float32 {
            return Err(ArrowSchemaError::WrongType {
                name: name.to_string(),
                dt,
            });
        }
        if col.len() != n {
            return Err(ArrowSchemaError::WrongLen {
                name: name.to_string(),
                expected: n,
                actual: col.len(),
            });
        }
    }
    Ok(())
}

pub fn col_f32<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a Float32Array, ArrowSchemaError> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| ArrowSchemaError::MissingColumn(name.to_string()))?;
    col.as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| ArrowSchemaError::WrongType {
            name: name.to_string(),
            dt: col.data_type().clone(),
        })
}
