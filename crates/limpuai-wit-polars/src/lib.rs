wit_bindgen::generate!({
    world: "polars-dataframe",
    path: "../../wit",
});

use std::cell::RefCell;
use std::io::Cursor;

use exports::limpuai::data::polars_parser::{
    AggFn, Aggregation, FilterOp, Guest, GuestDataframe, JoinType, SortOption,
};
use limpuai::data::types::{DataTable, FieldValue, SchemaField};
use polars::prelude::*;

// Import the generated resource handle type so we can return it from trait methods.
use exports::limpuai::data::polars_parser::Dataframe as DataframeHandle;

struct Component;

impl Guest for Component {
    type Dataframe = DataframeImpl;

    fn parse_csv(data: Vec<u8>) -> Result<DataframeHandle, String> {
        let cursor = Cursor::new(data);
        let df = CsvReader::new(cursor)
            .finish()
            .map_err(|e| format!("CSV parse error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(df)))
    }

    fn parse_json(data: Vec<u8>) -> Result<DataframeHandle, String> {
        let cursor = Cursor::new(data);
        let df = JsonReader::new(cursor)
            .finish()
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(df)))
    }

    fn join(
        left: DataframeHandle,
        right: DataframeHandle,
        left_on: Vec<String>,
        right_on: Vec<String>,
        how: JoinType,
    ) -> Result<DataframeHandle, String> {
        let left_df = left.get::<DataframeImpl>().inner();
        let right_df = right.get::<DataframeImpl>().inner();

        let join_type = match how {
            JoinType::Inner => polars::prelude::JoinType::Inner,
            JoinType::Left => polars::prelude::JoinType::Left,
            JoinType::Right => polars::prelude::JoinType::Right,
            JoinType::Full => polars::prelude::JoinType::Full,
        };

        let args = JoinArgs::new(join_type);
        let result = left_df
            .join(&right_df, left_on, right_on, args, None)
            .map_err(|e| format!("Join error: {e}"))?;

        Ok(DataframeHandle::new(DataframeImpl::new(result)))
    }
}

/// Internal wrapper around a Polars DataFrame.
///
/// Uses `RefCell` for interior mutability since WIT resource methods take `&self`.
struct DataframeImpl {
    inner: RefCell<DataFrame>,
}

impl DataframeImpl {
    fn new(df: DataFrame) -> Self {
        Self {
            inner: RefCell::new(df),
        }
    }

    fn inner(&self) -> std::cell::Ref<'_, DataFrame> {
        self.inner.borrow()
    }
}

impl GuestDataframe for DataframeImpl {
    fn columns(&self) -> Vec<SchemaField> {
        let df = self.inner();
        df.columns()
            .iter()
            .map(|c: &Column| SchemaField {
                name: c.name().to_string(),
                data_type: format!("{:?}", c.dtype()),
            })
            .collect()
    }

    fn height(&self) -> u64 {
        self.inner().height() as u64
    }

    fn width(&self) -> u64 {
        self.inner().width() as u64
    }

    fn select(&self, columns: Vec<String>) -> Result<DataframeHandle, String> {
        let df = self.inner();
        let selected = df
            .select(columns.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| format!("Select error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(selected)))
    }

    fn filter(
        &self,
        column: String,
        op: FilterOp,
        value: FieldValue,
    ) -> Result<DataframeHandle, String> {
        let df = self.inner();
        let col = df
            .column(&column)
            .map_err(|e| format!("Column '{}' not found: {e}", column))?;

        let mask = build_filter_mask(col.as_materialized_series(), op, &value)?;
        let filtered = df
            .filter(&mask)
            .map_err(|e| format!("Filter error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(filtered)))
    }

    fn sort(&self, by: Vec<SortOption>) -> Result<DataframeHandle, String> {
        let df = self.inner();
        let columns: Vec<String> = by.iter().map(|o| o.column.clone()).collect();
        let descending: Vec<bool> = by.iter().map(|o| o.descending).collect();
        let nulls_last: Vec<bool> = by.iter().map(|o| o.null_last).collect();

        let sort_options = SortMultipleOptions::new()
            .with_order_descending_multi(descending)
            .with_nulls_last_multi(nulls_last);

        let sorted = df
            .sort(
                columns.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                sort_options,
            )
            .map_err(|e| format!("Sort error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(sorted)))
    }

    fn head(&self, n: u64) -> Result<DataframeHandle, String> {
        let df = self.inner();
        Ok(DataframeHandle::new(DataframeImpl::new(df.head(Some(n as usize)))))
    }

    fn tail(&self, n: u64) -> Result<DataframeHandle, String> {
        let df = self.inner();
        Ok(DataframeHandle::new(DataframeImpl::new(df.tail(Some(n as usize)))))
    }

    fn unique(&self, subset: Option<Vec<String>>) -> Result<DataframeHandle, String> {
        let df = self.inner();
        let result = df
            .unique_impl(
                false,
                subset.map(|v| v.iter().map(|x| PlSmallStr::from_str(x.as_str())).collect()),
                UniqueKeepStrategy::First,
                None,
            )
            .map_err(|e| format!("Unique error: {e}"))?;
        Ok(DataframeHandle::new(DataframeImpl::new(result)))
    }

    fn group_by(
        &self,
        by: Vec<String>,
        aggregations: Vec<Aggregation>,
    ) -> Result<DataframeHandle, String> {
        let df = self.inner();

        // Perform the group_by first
        let gb = df
            .group_by(by.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| format!("Group-by error: {e}"))?;

        // For each aggregation, select the column and apply the function.
        // We run each aggregation separately and then stitch the results together.
        let keys = gb.keys();

        let mut result_columns: Vec<Column> = keys;

        for agg in &aggregations {
            let agg_df = apply_agg(
                gb.clone(),
                &agg.column,
                &agg.function,
                &agg.alias,
            );

            // The aggregation result has key columns + the aggregated column.
            // We only need the aggregated column (the last one).
            let cols: &[Column] = agg_df.columns();
            if let Some(last) = cols.last() {
                result_columns.push(last.clone());
            }
        }

        let height = result_columns
            .first()
            .map(|c: &Column| c.len())
            .unwrap_or(0);
        let result = unsafe { DataFrame::new_unchecked(height, result_columns) };

        Ok(DataframeHandle::new(DataframeImpl::new(result)))
    }

    fn to_table(&self) -> Result<DataTable, String> {
        let df = self.inner();
        dataframe_to_table(&df)
    }
}

/// Apply an aggregation function on a GroupBy and optionally rename the result column.
#[allow(deprecated)]
fn apply_agg(
    gb: GroupBy<'_>,
    column: &str,
    agg_fn: &AggFn,
    alias: &Option<String>,
) -> DataFrame {
    use AggFn::*;
    let mut df = match agg_fn {
        Sum => gb.select([column]).sum(),
        Mean => gb.select([column]).mean(),
        Min => gb.select([column]).min(),
        Max => gb.select([column]).max(),
        Count => gb.select([column]).count(),
        First => gb.select([column]).first(),
        Last => gb.select([column]).last(),
    }
    .unwrap_or_else(|_| DataFrame::empty());

    // Rename the last (non-key) column if alias is provided
    if let Some(alias_name) = alias {
        let cols: &[Column] = df.columns();
        if !cols.is_empty() {
            let last_idx = cols.len() - 1;
            let old_name = cols[last_idx].name().clone();
            let _ = cols;
            let _ = df.rename(&old_name, PlSmallStr::from_str(alias_name.as_str()));
        }
    }

    df
}

fn build_filter_mask(
    col: &Series,
    op: FilterOp,
    value: &FieldValue,
) -> Result<BooleanChunked, String> {
    use FilterOp::*;

    let mask = match value {
        FieldValue::Numeric(n) => {
            // Cast to f64 and compare — bind the cast result to avoid temporary lifetime issue
            let casted = col
                .cast(&DataType::Float64)
                .map_err(|e| format!("Cast error: {e}"))?;
            let ca = casted
                .f64()
                .map_err(|e| format!("Column is not numeric: {e}"))?;
            match op {
                Eq => ca.equal(*n),
                Neq => ca.not_equal(*n),
                Gt => ca.gt(*n),
                Gte => ca.gt_eq(*n),
                Lt => ca.lt(*n),
                Lte => ca.lt_eq(*n),
            }
        }
        FieldValue::Text(s) => {
            let ca = col
                .str()
                .map_err(|e| format!("Column is not string: {e}"))?;
            match op {
                Eq => ca.equal(s.as_str()),
                Neq => ca.not_equal(s.as_str()),
                Gt => ca.gt(s.as_str()),
                Gte => ca.gt_eq(s.as_str()),
                Lt => ca.lt(s.as_str()),
                Lte => ca.lt_eq(s.as_str()),
            }
        }
        FieldValue::Boolean(b) => {
            let ca = col
                .bool()
                .map_err(|e| format!("Column is not boolean: {e}"))?;
            // BooleanChunked comparison requires &BooleanChunked, not bool.
            // Create a single-element BooleanChunked for comparison.
            let scalar = BooleanChunked::new(PlSmallStr::EMPTY, &[*b]);
            match op {
                Eq => ca.equal(&scalar),
                Neq => ca.not_equal(&scalar),
                _ => return Err("Only eq/neq supported for boolean filter".into()),
            }
        }
        _ => return Err("Unsupported field value type for filter".into()),
    };
    Ok(mask)
}

fn dataframe_to_table(df: &DataFrame) -> Result<DataTable, String> {
    let columns: Vec<SchemaField> = df
        .columns()
        .iter()
        .map(|c: &Column| SchemaField {
            name: c.name().to_string(),
            data_type: format!("{:?}", c.dtype()),
        })
        .collect();

    let mut rows = Vec::new();
    for row_idx in 0..df.height() {
        let mut row = Vec::with_capacity(df.width());
        for col in df.columns() {
            let s: &Series = col.as_materialized_series();
            row.push(extract_field_value(s, row_idx));
        }
        rows.push(row);
    }

    Ok(DataTable { columns, rows })
}

fn extract_field_value(series: &Series, index: usize) -> FieldValue {
    use polars::datatypes::DataType;

    // Use Series::get to get AnyValue and check for null
    let av = match series.get(index) {
        Ok(av) => av,
        Err(_) => return FieldValue::Null,
    };

    // Check for null via AnyValue
    if matches!(av, AnyValue::Null) {
        return FieldValue::Null;
    }

    match series.dtype() {
        DataType::Boolean => {
            let ca = series.bool().expect("bool dtype");
            FieldValue::Boolean(ca.get(index).unwrap())
        }
        dt if dt.is_integer() => {
            let casted = series.cast(&DataType::Float64).unwrap();
            let ca = casted.f64().expect("cast to f64");
            FieldValue::Numeric(ca.get(index).unwrap())
        }
        DataType::Float32 | DataType::Float64 => {
            let ca = series.f64().expect("float64 dtype");
            FieldValue::Numeric(ca.get(index).unwrap())
        }
        DataType::String => {
            let ca = series.str().expect("string dtype");
            FieldValue::Text(ca.get(index).unwrap().to_string())
        }
        DataType::Date => {
            let ca = series.date().expect("date dtype");
            let days = ca.phys.get(index).unwrap();
            FieldValue::Timestamp(days as f64 * 86_400_000.0)
        }
        DataType::Datetime(tu, _tz) => {
            let ca = series.datetime().expect("datetime dtype");
            let v = ca.phys.get(index).unwrap();
            FieldValue::Timestamp(match tu {
                TimeUnit::Milliseconds => v as f64,
                TimeUnit::Microseconds => v as f64 / 1_000.0,
                TimeUnit::Nanoseconds => v as f64 / 1_000_000.0,
            })
        }
        DataType::Duration(tu) => {
            let ca = series.duration().expect("duration dtype");
            let v = ca.phys.get(index).unwrap();
            let ms = match tu {
                TimeUnit::Milliseconds => v as f64,
                TimeUnit::Microseconds => v as f64 / 1_000.0,
                TimeUnit::Nanoseconds => v as f64 / 1_000_000.0,
            };
            FieldValue::Numeric(ms)
        }
        DataType::Time => {
            let ca = series.time().expect("time dtype");
            let v = ca.phys.get(index).unwrap();
            FieldValue::Numeric(v as f64)
        }
        _ => FieldValue::Null,
    }
}

export!(Component);

/// Tests exercise the underlying Polars operations and conversion functions
/// directly, because `DataframeHandle::new()` requires the WASM component runtime.
/// This tests all the same logic that the WIT interface delegates to.
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper CSV data: id(Int64), name(String), score(Float64), active(Boolean), note(String with nulls)
    fn make_csv_data() -> Vec<u8> {
        b"id,name,score,active,note\n1,alice,95.5,true,ok\n2,bob,87.0,false,\n3,charlie,92.3,true,good\n"
            .to_vec()
    }

    /// CSV data with duplicate categories for group_by tests
    fn make_group_csv() -> Vec<u8> {
        b"category,value\nA,10\nB,20\nA,30\nB,40\n".to_vec()
    }

    /// CSV data with duplicate rows for unique tests
    fn make_dup_csv() -> Vec<u8> {
        b"id,name\n1,alice\n2,bob\n1,alice\n3,charlie\n".to_vec()
    }

    /// Parse CSV bytes into a Polars DataFrame
    fn csv_df(data: &[u8]) -> DataFrame {
        CsvReader::new(Cursor::new(data.to_vec()))
            .finish()
            .unwrap()
    }

    /// Parse JSON bytes into a Polars DataFrame
    fn json_df(data: &[u8]) -> DataFrame {
        JsonReader::new(Cursor::new(data.to_vec()))
            .finish()
            .unwrap()
    }

    /// Convert a DataFrame to a DataTable for assertions
    fn to_table(df: &DataFrame) -> DataTable {
        dataframe_to_table(df).unwrap()
    }

    // ── CSV Parsing ───────────────────────────────────────────────────

    #[test]
    fn parse_csv_basic() {
        let df = csv_df(&make_csv_data());
        let impl_ = DataframeImpl::new(df);

        assert_eq!(impl_.height(), 3);
        assert_eq!(impl_.width(), 5);

        let cols = impl_.columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "id");
        assert_eq!(cols[1].name, "name");
        assert_eq!(cols[2].name, "score");
        assert_eq!(cols[3].name, "active");
        assert_eq!(cols[4].name, "note");

        let table = impl_.to_table().unwrap();
        assert_eq!(table.rows.len(), 3);

        let r0 = &table.rows[0];
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 1.0));
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "alice"));
        assert!(matches!(&r0[2], FieldValue::Numeric(v) if (*v - 95.5).abs() < 0.01));
        assert!(matches!(&r0[3], FieldValue::Boolean(b) if *b));
        assert!(matches!(&r0[4], FieldValue::Text(s) if s == "ok"));
    }

    #[test]
    fn parse_csv_empty() {
        let df = csv_df(b"id,name\n");
        let impl_ = DataframeImpl::new(df);

        assert_eq!(impl_.height(), 0);
        assert_eq!(impl_.width(), 2);
    }

    #[test]
    fn parse_csv_invalid() {
        let result = CsvReader::new(Cursor::new(Vec::<u8>::new())).finish();
        assert!(result.is_err());
    }

    // ── JSON Parsing ──────────────────────────────────────────────────

    #[test]
    fn parse_json_basic() {
        let json = r#"[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]"#;
        let df = json_df(json.as_bytes());
        let impl_ = DataframeImpl::new(df);

        assert_eq!(impl_.height(), 2);
        assert_eq!(impl_.width(), 2);

        let table = impl_.to_table().unwrap();
        assert_eq!(table.rows.len(), 2);

        let r0 = &table.rows[0];
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 1.0));
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "alice"));
    }

    #[test]
    fn parse_ndjson() {
        let ndjson = "{\"id\":1,\"name\":\"alice\"}\n{\"id\":2,\"name\":\"bob\"}\n";
        let df = JsonReader::new(Cursor::new(ndjson.as_bytes().to_vec()))
            .with_json_format(polars::prelude::JsonFormat::JsonLines)
            .finish()
            .unwrap();
        let impl_ = DataframeImpl::new(df);

        assert_eq!(impl_.height(), 2);

        let table = impl_.to_table().unwrap();
        assert_eq!(table.rows.len(), 2);

        let r0 = &table.rows[0];
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 1.0));
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "alice"));
    }

    #[test]
    fn parse_json_invalid() {
        let result = JsonReader::new(Cursor::new(b"not json at all".to_vec())).finish();
        assert!(result.is_err());
    }

    // ── DataFrame Operations ──────────────────────────────────────────

    #[test]
    fn select_columns() {
        let df = csv_df(&make_csv_data());
        let selected = df
            .select(["id", "name"])
            .expect("select should succeed");

        assert_eq!(selected.width(), 2);
        assert_eq!(selected.get_column_names(), &["id", "name"]);

        let table = to_table(&selected);
        assert_eq!(table.rows.len(), 3);
    }

    #[test]
    fn select_missing_column() {
        let df = csv_df(&make_csv_data());
        let result = df.select(["nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn filter_numeric() {
        let df = csv_df(&make_csv_data());
        let col = df.column("id").expect("column exists");
        let mask = build_filter_mask(
            col.as_materialized_series(),
            FilterOp::Gt,
            &FieldValue::Numeric(1.0),
        )
        .unwrap();
        let filtered = df.filter(&mask).unwrap();
        let table = to_table(&filtered);

        assert_eq!(table.rows.len(), 2);
        // bob (id=2) and charlie (id=3)
        assert!(matches!(&table.rows[0][0], FieldValue::Numeric(v) if *v == 2.0));
        assert!(matches!(&table.rows[1][0], FieldValue::Numeric(v) if *v == 3.0));
    }

    #[test]
    fn filter_string() {
        let df = csv_df(&make_csv_data());
        let col = df.column("name").expect("column exists");
        let mask = build_filter_mask(
            col.as_materialized_series(),
            FilterOp::Eq,
            &FieldValue::Text("alice".into()),
        )
        .unwrap();
        let filtered = df.filter(&mask).unwrap();
        let table = to_table(&filtered);

        assert_eq!(table.rows.len(), 1);
        assert!(matches!(&table.rows[0][1], FieldValue::Text(s) if s == "alice"));
    }

    #[test]
    fn filter_boolean() {
        let df = csv_df(&make_csv_data());
        let col = df.column("active").expect("column exists");
        let mask = build_filter_mask(
            col.as_materialized_series(),
            FilterOp::Eq,
            &FieldValue::Boolean(true),
        )
        .unwrap();
        let filtered = df.filter(&mask).unwrap();
        let table = to_table(&filtered);

        assert_eq!(table.rows.len(), 2);
        // alice (active=true) and charlie (active=true)
        assert!(matches!(&table.rows[0][1], FieldValue::Text(s) if s == "alice"));
        assert!(matches!(&table.rows[1][1], FieldValue::Text(s) if s == "charlie"));
    }

    #[test]
    fn sort_ascending() {
        let df = csv_df(&make_csv_data());
        let sort_opts = SortMultipleOptions::new()
            .with_order_descending_multi([false])
            .with_nulls_last_multi([false]);
        let sorted = df.sort(["score"], sort_opts).unwrap();
        let table = to_table(&sorted);

        assert_eq!(table.rows.len(), 3);
        assert!(matches!(&table.rows[0][2], FieldValue::Numeric(v) if (*v - 87.0).abs() < 0.01));
        assert!(matches!(&table.rows[1][2], FieldValue::Numeric(v) if (*v - 92.3).abs() < 0.01));
        assert!(matches!(&table.rows[2][2], FieldValue::Numeric(v) if (*v - 95.5).abs() < 0.01));
    }

    #[test]
    fn sort_descending() {
        let df = csv_df(&make_csv_data());
        let sort_opts = SortMultipleOptions::new()
            .with_order_descending_multi([true])
            .with_nulls_last_multi([false]);
        let sorted = df.sort(["score"], sort_opts).unwrap();
        let table = to_table(&sorted);

        assert_eq!(table.rows.len(), 3);
        assert!(matches!(&table.rows[0][2], FieldValue::Numeric(v) if (*v - 95.5).abs() < 0.01));
        assert!(matches!(&table.rows[1][2], FieldValue::Numeric(v) if (*v - 92.3).abs() < 0.01));
        assert!(matches!(&table.rows[2][2], FieldValue::Numeric(v) if (*v - 87.0).abs() < 0.01));
    }

    #[test]
    fn head_tail() {
        let df = csv_df(&make_csv_data());

        let head = df.head(Some(2));
        let head_table = to_table(&head);
        assert_eq!(head_table.rows.len(), 2);
        assert!(matches!(&head_table.rows[0][1], FieldValue::Text(s) if s == "alice"));
        assert!(matches!(&head_table.rows[1][1], FieldValue::Text(s) if s == "bob"));

        let tail = df.tail(Some(1));
        let tail_table = to_table(&tail);
        assert_eq!(tail_table.rows.len(), 1);
        assert!(matches!(&tail_table.rows[0][1], FieldValue::Text(s) if s == "charlie"));
    }

    #[test]
    fn unique_rows() {
        let df = csv_df(&make_dup_csv());
        let result = df
            .unique_impl(false, None, UniqueKeepStrategy::First, None)
            .unwrap();

        // 4 rows with one duplicate → 3 unique
        assert_eq!(result.height(), 3);
    }

    #[test]
    fn unique_subset() {
        let df = csv_df(&make_dup_csv());
        let subset = Some(
            ["name"]
                .iter()
                .map(|s| PlSmallStr::from_str(s))
                .collect(),
        );
        let result = df
            .unique_impl(false, subset, UniqueKeepStrategy::First, None)
            .unwrap();

        // alice appears once even though id=1 row appears twice
        assert_eq!(result.height(), 3);
    }

    // ── Group By + Aggregation ────────────────────────────────────────

    #[test]
    fn group_by_sum() {
        let df = csv_df(&make_group_csv());
        let gb = df.group_by(["category"]).unwrap();
        let agg = gb.select(["value"]).sum().unwrap();
        let table = to_table(&agg);

        assert_eq!(table.rows.len(), 2);
        // A: 10+30=40, B: 20+40=60
        let a_row = table
            .rows
            .iter()
            .find(|r| matches!(&r[0], FieldValue::Text(s) if s == "A"))
            .unwrap();
        assert!(matches!(&a_row[1], FieldValue::Numeric(v) if *v == 40.0));

        let b_row = table
            .rows
            .iter()
            .find(|r| matches!(&r[0], FieldValue::Text(s) if s == "B"))
            .unwrap();
        assert!(matches!(&b_row[1], FieldValue::Numeric(v) if *v == 60.0));
    }

    #[test]
    fn group_by_multiple_agg() {
        let df = csv_df(&make_group_csv());
        let gb = df.group_by(["category"]).unwrap();

        let sum_df = gb.clone().select(["value"]).sum().unwrap();
        let mean_df = gb.select(["value"]).mean().unwrap();

        // Build combined result: category + sum + mean
        let keys = sum_df.column("category").unwrap().clone();
        let sum_col = sum_df.columns().last().unwrap().clone();
        let mean_col = mean_df.columns().last().unwrap().clone();

        let combined =
            unsafe { DataFrame::new_unchecked(keys.len(), vec![keys, sum_col, mean_col]) };
        assert_eq!(combined.height(), 2);
        assert_eq!(combined.width(), 3); // category + sum + mean
    }

    #[test]
    fn group_by_with_alias() {
        let df = csv_df(&make_group_csv());
        let gb = df.group_by(["category"]).unwrap();
        let mut agg = gb.select(["value"]).sum().unwrap();

        // Rename last column to alias
        let last_name = agg.columns().last().unwrap().name().clone();
        agg.rename(&last_name, PlSmallStr::from_str("total")).unwrap();

        let names = agg.get_column_names();
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|n| *n == "total"));
    }

    // ── Join ──────────────────────────────────────────────────────────

    #[test]
    fn join_inner() {
        let left = csv_df(b"id,name\n1,alice\n2,bob\n3,charlie\n");
        let right = csv_df(b"id,score\n1,95.5\n2,87.0\n");

        let args = JoinArgs::new(polars::prelude::JoinType::Inner);
        let joined = left
            .join(&right, ["id"], ["id"], args, None)
            .unwrap();

        assert_eq!(joined.height(), 2);
    }

    #[test]
    fn join_left() {
        let left = csv_df(b"id,name\n1,alice\n2,bob\n3,charlie\n");
        let right = csv_df(b"id,score\n1,95.5\n2,87.0\n");

        let args = JoinArgs::new(polars::prelude::JoinType::Left);
        let joined = left
            .join(&right, ["id"], ["id"], args, None)
            .unwrap();

        // All left rows: alice, bob, charlie (charlie has null score)
        assert_eq!(joined.height(), 3);
    }

    #[test]
    fn join_full() {
        let left = csv_df(b"id,name\n1,alice\n2,bob\n");
        let right = csv_df(b"id,score\n1,95.5\n3,99.0\n");

        let args = JoinArgs::new(polars::prelude::JoinType::Full);
        let joined = left
            .join(&right, ["id"], ["id"], args, None)
            .unwrap();

        // alice(1), bob(2), null/name(3)
        assert_eq!(joined.height(), 3);
    }

    // ── to_table Export ────────────────────────────────────────────────

    #[test]
    fn to_table_types() {
        let df = csv_df(&make_csv_data());
        let table = to_table(&df);

        let r0 = &table.rows[0];
        // Int64 → Numeric
        assert!(matches!(&r0[0], FieldValue::Numeric(v) if *v == 1.0));
        // String → Text
        assert!(matches!(&r0[1], FieldValue::Text(s) if s == "alice"));
        // Float64 → Numeric
        assert!(matches!(&r0[2], FieldValue::Numeric(_)));
        // Boolean → Boolean
        assert!(matches!(&r0[3], FieldValue::Boolean(b) if *b));
        // String → Text
        assert!(matches!(&r0[4], FieldValue::Text(s) if s == "ok"));

        // Row 1: bob has empty note in CSV — verify null or empty string mapping
        let r1 = &table.rows[1];
        assert!(
            matches!(&r1[4], FieldValue::Null)
                || matches!(&r1[4], FieldValue::Text(s) if s.is_empty())
        );
    }

    // ── Operation Chain ────────────────────────────────────────────────

    #[test]
    fn chain_filter_sort_head() {
        let df = csv_df(&make_csv_data());

        // Filter: active == true
        let col = df.column("active").expect("column exists");
        let mask = build_filter_mask(
            col.as_materialized_series(),
            FilterOp::Eq,
            &FieldValue::Boolean(true),
        )
        .unwrap();
        let filtered = df.filter(&mask).unwrap();

        // Sort by score descending
        let sort_opts = SortMultipleOptions::new()
            .with_order_descending_multi([true])
            .with_nulls_last_multi([false]);
        let sorted = filtered.sort(["score"], sort_opts).unwrap();

        // Head 1
        let top = sorted.head(Some(1));
        let table = to_table(&top);

        assert_eq!(table.rows.len(), 1);
        // alice (95.5) > charlie (92.3)
        assert!(matches!(&table.rows[0][1], FieldValue::Text(s) if s == "alice"));
        assert!(
            matches!(&table.rows[0][2], FieldValue::Numeric(v) if (*v - 95.5).abs() < 0.01)
        );
    }
}
