wit_bindgen::generate!({
    world: "arrow-ipc",
    path: "../../wit",
});

use arrow::array::*;
use arrow::datatypes::*;
use arrow::ipc::reader::{FileReader, StreamReader};
use exports::limpuai::data::arrow_parser::Guest;
use limpuai::data::types::{DataTable, FieldValue, SchemaField};

struct Component;

impl Guest for Component {
    fn parse(data: Vec<u8>) -> Result<DataTable, String> {
        std::panic::catch_unwind(|| parse_inner(&data))
            .map_err(|_| "Arrow IPC parse panic: invalid data".to_string())?
    }
}

fn parse_inner(data: &[u8]) -> Result<DataTable, String> {
    let batches = match FileReader::try_new(std::io::Cursor::new(data), None) {
        Ok(reader) => reader
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Arrow IPC file read error: {e}"))?,
        Err(_) => {
            let reader = StreamReader::try_new(std::io::Cursor::new(data), None)
                .map_err(|e| format!("Arrow IPC stream error: {e}"))?;
            reader
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Arrow IPC read error: {e}"))?
        }
    };

    if batches.is_empty() {
        return Ok(DataTable {
            columns: vec![],
            rows: vec![],
        });
    }

    let schema = batches[0].schema();
    let columns: Vec<SchemaField> = schema
        .fields()
        .iter()
        .map(|f| SchemaField {
            name: f.name().clone(),
            data_type: format!("{:?}", f.data_type()),
        })
        .collect();

    let mut rows = Vec::new();
    for batch in &batches {
        for row_idx in 0..batch.num_rows() {
            let mut row = Vec::with_capacity(batch.num_columns());
            for col_idx in 0..batch.num_columns() {
                row.push(extract_field_value(batch.column(col_idx), row_idx));
            }
            rows.push(row);
        }
    }

    Ok(DataTable { columns, rows })
}

fn extract_field_value(array: &dyn Array, index: usize) -> FieldValue {
    if array.is_null(index) {
        return FieldValue::Null;
    }

    match array.data_type() {
        DataType::Boolean => FieldValue::Boolean(array.as_boolean().value(index)),
        DataType::Int8 => {
            FieldValue::Numeric(array.as_primitive::<Int8Type>().value(index) as f64)
        }
        DataType::Int16 => {
            FieldValue::Numeric(array.as_primitive::<Int16Type>().value(index) as f64)
        }
        DataType::Int32 => {
            FieldValue::Numeric(array.as_primitive::<Int32Type>().value(index) as f64)
        }
        DataType::Int64 => {
            FieldValue::Numeric(array.as_primitive::<Int64Type>().value(index) as f64)
        }
        DataType::UInt8 => {
            FieldValue::Numeric(array.as_primitive::<UInt8Type>().value(index) as f64)
        }
        DataType::UInt16 => {
            FieldValue::Numeric(array.as_primitive::<UInt16Type>().value(index) as f64)
        }
        DataType::UInt32 => {
            FieldValue::Numeric(array.as_primitive::<UInt32Type>().value(index) as f64)
        }
        DataType::UInt64 => {
            FieldValue::Numeric(array.as_primitive::<UInt64Type>().value(index) as f64)
        }
        DataType::Float32 => {
            FieldValue::Numeric(array.as_primitive::<Float32Type>().value(index) as f64)
        }
        DataType::Float64 => {
            FieldValue::Numeric(array.as_primitive::<Float64Type>().value(index) as f64)
        }
        DataType::Utf8 => {
            FieldValue::Text(array.as_string::<i32>().value(index).to_string())
        }
        DataType::LargeUtf8 => {
            FieldValue::Text(array.as_string::<i64>().value(index).to_string())
        }
        DataType::Date32 => {
            let days = array.as_primitive::<Date32Type>().value(index);
            FieldValue::Timestamp(days as f64 * 86_400_000.0)
        }
        DataType::Date64 => {
            let ms = array.as_primitive::<Date64Type>().value(index);
            FieldValue::Timestamp(ms as f64)
        }
        DataType::Timestamp(TimeUnit::Second, _) => {
            let v = array.as_primitive::<TimestampSecondType>().value(index);
            FieldValue::Timestamp(v as f64 * 1000.0)
        }
        DataType::Timestamp(TimeUnit::Millisecond, _) => {
            let v = array.as_primitive::<TimestampMillisecondType>().value(index);
            FieldValue::Timestamp(v as f64)
        }
        DataType::Timestamp(TimeUnit::Microsecond, _) => {
            let v = array.as_primitive::<TimestampMicrosecondType>().value(index);
            FieldValue::Timestamp(v as f64 / 1000.0)
        }
        DataType::Timestamp(TimeUnit::Nanosecond, _) => {
            let v = array.as_primitive::<TimestampNanosecondType>().value(index);
            FieldValue::Timestamp(v as f64 / 1_000_000.0)
        }
        DataType::Null => FieldValue::Null,
        _ => FieldValue::Null,
    }
}

export!(Component);

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::ipc::writer::StreamWriter;
    use arrow::record_batch::RecordBatch;

    fn make_batch() -> RecordBatch {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("score", DataType::Float64, false),
            Field::new("active", DataType::Boolean, false),
            Field::new("ts", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("note", DataType::Utf8, true),
        ]);
        RecordBatch::try_new(
            std::sync::Arc::new(schema),
            vec![
                std::sync::Arc::new(Int64Array::from(vec![1, 2, 3])),
                std::sync::Arc::new(StringArray::from(vec!["alice", "bob", "charlie"])),
                std::sync::Arc::new(Float64Array::from(vec![95.5, 87.0, 92.3])),
                std::sync::Arc::new(BooleanArray::from(vec![true, false, true])),
                std::sync::Arc::new(TimestampMillisecondArray::from(vec![
                    1700000000000i64,
                    1700000001000,
                    1700000002000,
                ])),
                std::sync::Arc::new(StringArray::from(vec![
                    Some("ok"),
                    None,
                    Some("good"),
                ])),
            ],
        )
        .unwrap()
    }

    fn write_ipc_stream(batch: &RecordBatch) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut writer = StreamWriter::try_new(&mut buf, &batch.schema()).unwrap();
        writer.write(batch).unwrap();
        writer.finish().unwrap();
        buf
    }

    #[test]
    fn parse_stream() {
        let batch = make_batch();
        let data = write_ipc_stream(&batch);
        let table = Component::parse(data).expect("parse should succeed");

        assert_eq!(table.columns.len(), 6);
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.rows.len(), 3);

        let r0 = &table.rows[0];
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 1.0));
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "alice"));
        assert!(matches!(&r0[3], FieldValue::Boolean(b) if *b));
        assert!(matches!(&r0[4], FieldValue::Timestamp(v) if *v == 1700000000000.0));
        assert!(matches!(&r0[5], FieldValue::Text(s) if s == "ok"));
    }

    #[test]
    fn null_values() {
        let batch = make_batch();
        let table = Component::parse(write_ipc_stream(&batch)).unwrap();
        assert!(matches!(&table.rows[1][5], FieldValue::Null));
    }

    #[test]
    fn empty_stream() {
        let schema = Schema::new(vec![Field::new("x", DataType::Int32, false)]);
        let mut buf = Vec::new();
        let mut w = StreamWriter::try_new(&mut buf, &std::sync::Arc::new(schema)).unwrap();
        w.finish().unwrap();
        let table = Component::parse(buf).unwrap();
        assert_eq!(table.rows.len(), 0);
    }

    #[test]
    fn invalid_data() {
        assert!(Component::parse(vec![0xDE, 0xAD, 0xBE, 0xEF]).is_err());
    }
}
