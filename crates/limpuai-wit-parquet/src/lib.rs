wit_bindgen::generate!({
    world: "parquet-file",
    path: "../../wit",
});

use bytes::Bytes;
use exports::limpuai::data::parquet_parser::Guest;
use limpuai::data::types::{DataTable, FieldValue, SchemaField};
use parquet::file::reader::{FileReader, SerializedFileReader};

struct Component;

impl Guest for Component {
    fn parse(data: Vec<u8>) -> Result<DataTable, String> {
        let bytes = Bytes::from(data);
        let reader = SerializedFileReader::new(bytes)
            .map_err(|e| format!("Parquet open error: {e}"))?;

        let schema = reader.metadata().file_metadata().schema_descr();
        let columns: Vec<SchemaField> = schema
            .columns()
            .iter()
            .map(|c| SchemaField {
                name: c.name().to_string(),
                data_type: format!("{:?}", c.physical_type()),
            })
            .collect();

        let mut rows = Vec::new();
        let row_iter = reader
            .get_row_iter(None)
            .map_err(|e| format!("Parquet row iterator error: {e}"))?;

        for row_result in row_iter {
            let row = row_result.map_err(|e| format!("Parquet row read error: {e}"))?;
            let columns = row.into_columns();
            rows.push(columns.into_iter().map(|(_name, field)| convert_field(field)).collect());
        }

        Ok(DataTable { columns, rows })
    }
}

fn convert_field(field: parquet::record::Field) -> FieldValue {
    match field {
        parquet::record::Field::Null => FieldValue::Null,
        parquet::record::Field::Bool(b) => FieldValue::Boolean(b),
        parquet::record::Field::Byte(b) => FieldValue::Numeric(b as f64),
        parquet::record::Field::Short(s) => FieldValue::Numeric(s as f64),
        parquet::record::Field::Int(i) => FieldValue::Numeric(i as f64),
        parquet::record::Field::Long(l) => FieldValue::Numeric(l as f64),
        parquet::record::Field::UByte(u) => FieldValue::Numeric(u as f64),
        parquet::record::Field::UShort(u) => FieldValue::Numeric(u as f64),
        parquet::record::Field::UInt(u) => FieldValue::Numeric(u as f64),
        parquet::record::Field::ULong(u) => FieldValue::Numeric(u as f64),
        parquet::record::Field::Float16(f) => FieldValue::Numeric(f.to_f64() as f64),
        parquet::record::Field::Float(f) => FieldValue::Numeric(f as f64),
        parquet::record::Field::Double(d) => FieldValue::Numeric(d),
        parquet::record::Field::Decimal(_) => FieldValue::Text("[decimal]".to_string()),
        parquet::record::Field::Str(s) => FieldValue::Text(s),
        parquet::record::Field::Bytes(b) => FieldValue::Text(format!("{:x?}", b.data())),
        parquet::record::Field::Date(days) => FieldValue::Timestamp(days as f64 * 86_400_000.0),
        parquet::record::Field::TimestampMillis(ms) => FieldValue::Timestamp(ms as f64),
        parquet::record::Field::TimestampMicros(us) => FieldValue::Timestamp(us as f64 / 1000.0),
        parquet::record::Field::Group(_) => FieldValue::Null,
        parquet::record::Field::ListInternal(_) => FieldValue::Null,
        parquet::record::Field::MapInternal(_) => FieldValue::Null,
    }
}

export!(Component);

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::*;
    use arrow::datatypes::*;
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::arrow_writer::ArrowWriter;
    use std::sync::Arc;

    fn write_parquet(batches: Vec<RecordBatch>) -> Vec<u8> {
        let schema = batches[0].schema();
        let mut buf = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut buf, schema, None).unwrap();
        for batch in batches {
            writer.write(&batch).unwrap();
        }
        writer.close().unwrap();
        buf
    }

    fn make_batch() -> RecordBatch {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("score", DataType::Float64, false),
            Field::new("active", DataType::Boolean, false),
            Field::new("note", DataType::Utf8, true),
        ]);
        RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![10, 20])),
                Arc::new(StringArray::from(vec!["x", "y"])),
                Arc::new(Float64Array::from(vec![1.5, 2.5])),
                Arc::new(BooleanArray::from(vec![true, false])),
                Arc::new(StringArray::from(vec![Some("yes"), None])),
            ],
        )
        .unwrap()
    }

    #[test]
    fn parse_basic() {
        let data = write_parquet(vec![make_batch()]);
        let table = Component::parse(data).expect("parse should succeed");

        assert_eq!(table.columns.len(), 5);
        assert_eq!(table.rows.len(), 2);

        let r0 = &table.rows[0];
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 10.0));
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "x"));
        assert!(matches!(&r0[2], FieldValue::Numeric(v) if *v == 1.5));
        assert!(matches!(&r0[3], FieldValue::Boolean(b) if *b));
        assert!(matches!(&r0[4], FieldValue::Text(s) if s == "yes"));
    }

    #[test]
    fn null_values() {
        let data = write_parquet(vec![make_batch()]);
        let table = Component::parse(data).unwrap();
        assert!(matches!(&table.rows[1][4], FieldValue::Null));
    }

    #[test]
    fn empty_file() {
        let schema = Schema::new(vec![Field::new("x", DataType::Int32, false)]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![Arc::new(Int32Array::from(Vec::<i32>::new()))],
        )
        .unwrap();
        let data = write_parquet(vec![batch]);
        let table = Component::parse(data).unwrap();
        assert_eq!(table.rows.len(), 0);
        assert_eq!(table.columns.len(), 1);
    }

    #[test]
    fn timestamp_types() {
        let schema = Schema::new(vec![
            Field::new("ts_ms", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("ts_us", DataType::Timestamp(TimeUnit::Microsecond, None), false),
        ]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(TimestampMillisecondArray::from(vec![1700000000000i64])),
                Arc::new(TimestampMicrosecondArray::from(vec![1700000000000000i64])),
            ],
        )
        .unwrap();
        let data = write_parquet(vec![batch]);
        let table = Component::parse(data).unwrap();

        assert!(matches!(&table.rows[0][0], FieldValue::Timestamp(v) if *v == 1700000000000.0));
        assert!(matches!(&table.rows[0][1], FieldValue::Timestamp(v) if *v == 1700000000000.0));
    }

    #[test]
    fn multiple_batches() {
        let batch1 = make_batch();
        let schema2 = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("score", DataType::Float64, false),
            Field::new("active", DataType::Boolean, false),
            Field::new("note", DataType::Utf8, true),
        ]);
        let batch2 = RecordBatch::try_new(
            Arc::new(schema2),
            vec![
                Arc::new(Int64Array::from(vec![30])),
                Arc::new(StringArray::from(vec!["z"])),
                Arc::new(Float64Array::from(vec![3.5])),
                Arc::new(BooleanArray::from(vec![true])),
                Arc::new(StringArray::from(vec![Some("ok")])),
            ],
        )
        .unwrap();
        let data = write_parquet(vec![batch1, batch2]);
        let table = Component::parse(data).unwrap();
        assert_eq!(table.rows.len(), 3);
        assert!(matches!(&table.rows[2][0], FieldValue::Numeric(v) if *v == 30.0));
    }

    #[test]
    fn invalid_data() {
        assert!(Component::parse(vec![0xDE, 0xAD, 0xBE, 0xEF]).is_err());
    }
}
