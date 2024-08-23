#![doc(html_root_url = "https://docs.rs/polars-sqlite/0.3.9")]
//! Rust sqlite3 traits for polars dataframe
//!

use std::error::Error;
use polars::{series::Series, prelude::{ChunkApply}}; // , NamedFrom
use polars::prelude::{DataFrame, AnyValue, Schema, DataType}; // , Field
use anyvalue_dataframe::{from_any, to_any};
use anyvalue_dataframe::{row_schema, named_schema};
use sqlite;

use itertools::Itertools;
use iter_tuple::{struct_derive, tuple_sqlite3, tuple_derive};

/// trait ToSqlite3ValueVec
pub trait ToSqlite3ValueVec {
  ///
  fn to_sqlite3_vec(&self) -> Vec<(&'_ str, sqlite::Value)>;
}

/// trait IntoAnyValueVec
pub trait IntoAnyValueVec<'a> {
  ///
  fn into_vec(self) -> Vec<AnyValue<'a>>;
}

/// create DataFrame from sl3
/// - n: &amp;Vec&lt;&amp;str&gt;
/// - p: &amp;[(&amp;str, sqlite::Value)]
/// - f: FnMut(&amp;'a sqlite::Row) -> Vec&lt;AnyValue&lt;'_&gt;&gt;
/// - df_from_sl3("db.sl3", n, "select * from tbl;", p, f).expect("df")
pub fn df_from_sl3<F>(dbn: &str, n: &Vec<&str>,
  qry: &str, p: &[(&str, sqlite::Value)],
  mut f: F) -> Result<DataFrame, Box<dyn Error>>
  where F: for<'a> FnMut(&'a sqlite::Row) -> Vec<AnyValue<'_>> {
  let cn = sqlite::open(dbn)?;
  let stmt = cn.prepare(qry)?;
  let rows: Vec<sqlite::Row> =
    stmt.into_iter().bind::<&[(_, sqlite::Value)]>(p)?
    .map(|row| row.expect("row")).collect();
  if rows.len() == 0 { return Ok(DataFrame::new(Vec::<Series>::new())?) }
  // convert row as sqlite::Row to polars::frame::row::Row
  let rows: Vec<polars::frame::row::Row> = rows.iter().map(|row|
    row_schema(f(row))).collect();
  let schema = Schema::from(&rows[0]);
  let mut df = DataFrame::from_rows_iter_and_schema(rows.iter(), &schema)
  .expect("create DataFrame"); // noname
  df.set_column_names(&n).expect("set column names");
  Ok(df)
}

/// create DataFrame from sl3 with column names and DataTypes
/// - n: &amp;Vec&lt;&amp;str&gt;
/// - t: &amp;Vec&lt;DataType&gt;
/// - p: &amp;[(&amp;str, sqlite::Value)]
/// - f: FnMut(&amp;'a sqlite::Row) -> Vec&lt;AnyValue&lt;'_&gt;&gt;
/// - df_from_sl3_type("db.sl3", n, t, "select * from tbl;", p, f).expect("df")
pub fn df_from_sl3_type<F>(dbn: &str, n: &Vec<&str>, t: &Vec<DataType>,
  qry: &str, p: &[(&str, sqlite::Value)],
  mut f: F) -> Result<DataFrame, Box<dyn Error>>
  where F: for<'a> FnMut(&'a sqlite::Row) -> Vec<AnyValue<'_>> {
  let sels = n.iter().enumerate().map(|(i, n)|
    // Series::new(n, &Vec::<AnyValue<'_>>::new()) // Int32 default (NamedFrom)
    // Series::from_any_values(n, &vec![]).expect("series") // Int32 default
    Series::from_any_values_and_dtype(n, &vec![], &t[i]).expect("series")
  ).collect::<Vec<_>>();
  let mut df = DataFrame::new(sels)?;
  // println!("{:?}", named_schema(&df, n.clone()));
  let cn = sqlite::open(dbn)?;
  let stmt = cn.prepare(qry)?;
  df = stmt.into_iter().bind::<&[(_, sqlite::Value)]>(p)?
  .map(|row| row.expect("row")).map(|row| {
    // convert row as sqlite::Row to polars::frame::row::Row
    // create temporary single element vec as single row
    let rows = vec![row_schema(f(&row))];
    let schema = Schema::from(&rows[0]);
    let mut s_df = DataFrame::from_rows_iter_and_schema(rows.iter(), &schema)
    .expect("create temporary DataFrame"); // noname
    s_df.set_column_names(&n).expect("set column names");
    s_df
  }).fold(df, |s, a| s.vstack(&a).expect("vstack"));
  // }).reduce(|s, a| s.vstack(&a).expect("vstack")).expect("reduce"); // len>0
  Ok(df)
}

/// cols
/// - an: (bool, usize) = (when bool is true, skip usize number)
pub fn sl3_cols(n: &Vec<&str>, an: (bool, usize)) -> String {
  n.iter().enumerate().map(|(i, s)|
    if an.0 && an.1 == i { None } else { Some(format!("{}", s)) }
  ).filter_map(|s| s).collect::<Vec<_>>().join(", ")
}

/// tags
/// - an: (bool, usize) = (when bool is true, skip usize number)
pub fn sl3_tags(n: &Vec<&str>, an: (bool, usize)) -> String {
  n.iter().enumerate().map(|(i, s)|
    if an.0 && an.1 == i { None } else { Some(format!(":{}", s)) }
  ).filter_map(|s| s).collect::<Vec<_>>().join(", ")
}

/// insert row
pub fn sl3_insert_row<T>(cn: &sqlite::Connection, qry: &str,
  row: &T, an: (bool, usize)) -> Result<(), Box<dyn Error>>
  where T: ToSqlite3ValueVec {
  let mut stmt = cn.prepare(qry)?;
  let mut s = row.to_sqlite3_vec();
  if an.0 { s.remove(an.1); } // will panic when out of index range
  stmt.bind_iter::<_, (_, sqlite::Value)>(s)?;
  stmt.next()?;
  Ok(())
}

/// insert
/// - an: (bool, usize) = (when bool is true, skip usize number)
pub fn sl3_insert<T>(dbn: &str, qry: &str,
  v: &Vec<T>, an: (bool, usize)) -> Result<(), Box<dyn Error>>
  where T: ToSqlite3ValueVec {
  let cn = sqlite::open(dbn)?;
  for row in v.iter() { sl3_insert_row::<T>(&cn, qry, row, an)? }
  Ok(())
}

/// insert df
pub fn sl3_insert_df<'a, F, G>(dbn: &str, qry: &str,
  df: &DataFrame, an: (bool, usize), mut f: F, mut g: G) ->
  Result<(), Box<dyn Error>> where
  F: FnMut(&sqlite::Connection, &str, &Vec<AnyValue<'_>>, (bool, usize)) ->
    Result<(), Box<dyn Error>>,
  G: FnMut() -> polars::frame::row::Row<'a> {
  let cn = sqlite::open(dbn)?;
  let mut row = g();
  for i in (0..df.height()).into_iter() {
    df.get_row_amortized(i, &mut row);
    f(&cn, qry, &row.0, an)?
  }
  Ok(())
}

/// asgns
pub fn sl3_asgns(n: &Vec<&str>) -> String {
  n.iter().map(|s| format!("{}=:{}", s, s)).collect::<Vec<_>>().join(", ")
}

/// kvs
pub fn sl3_kvs<'a, T>(row: &'a T, pick: &Vec<&str>) ->
  Vec<(&'a str, sqlite::Value)> where T: ToSqlite3ValueVec {
  let mut s = row.to_sqlite3_vec();
  for i in (0..s.len()).rev() { // rev index never out of range
    if !pick.contains(&&s[i].0[1..]) { s.remove(i); }
  }
  s
}

/// update row
pub fn sl3_update_row(cn: &sqlite::Connection, qry: &str,
  p: Vec<(&str, sqlite::Value)>) -> Result<(), Box<dyn Error>> {
  let mut stmt = cn.prepare(qry)?;
  stmt.bind_iter::<_, (_, sqlite::Value)>(p)?;
  stmt.next()?;
  Ok(())
}

/// update
/// - f: |cn, qry, row, pick| { sl3_update_row(cn, qry, sl3_kvs(row, pick)) }
pub fn sl3_update<T, F>(dbn: &str, qry: &str,
  v: &Vec<T>, pick: &Vec<&str>, mut f: F) -> Result<(), Box<dyn Error>> where
  T: ToSqlite3ValueVec,
  F: FnMut(&sqlite::Connection, &str, &T, &Vec<&str>) ->
    Result<(), Box<dyn Error>> {
  let cn = sqlite::open(dbn)?;
  for row in v.iter() { f(&cn, qry, row, pick)? }
  Ok(())
}

/// update df
pub fn sl3_update_df<'a, F, G>(dbn: &str, qry: &str,
  df: &DataFrame, pick: &Vec<&str>, mut f: F, mut g: G) ->
  Result<(), Box<dyn Error>> where
  F: FnMut(&sqlite::Connection, &str, &Vec<AnyValue<'_>>, &Vec<&str>) ->
    Result<(), Box<dyn Error>>,
  G: FnMut() -> polars::frame::row::Row<'a> {
  let cn = sqlite::open(dbn)?;
  let mut row = g();
  for i in (0..df.height()).into_iter() {
    df.get_row_amortized(i, &mut row);
    f(&cn, qry, &row.0, pick)?
  }
  Ok(())
}

/// to sqlite3 value
#[macro_export]
macro_rules! to_sl3 {
  (Int64, $v: expr) => { $v.into() };
  (Int32, $v: expr) => { ($v as i64).into() };
  (Int16, $v: expr) => { ($v as i64).into() };
  (Int8, $v: expr) => { ($v as i64).into() };
  (UInt64, $v: expr) => { ($v as i64).into() };
  (UInt32, $v: expr) => { ($v as i64).into() };
  (UInt16, $v: expr) => { ($v as i64).into() };
  (UInt8, $v: expr) => { ($v as i64).into() };
  (Float64, $v: expr) => { $v.into() }; // Decimal in polars latest
  (Float32, $v: expr) => { ($v as f64).into() }; // Decimal in polars latest
  (Utf8, $v: expr) => { $v.into() }; // polars version 0.25.1
  (String, $v: expr) => { $v.into() }; // polars latest
  (Boolean, $v: expr) => { (if $v {"T"} else {"F"}).into() };
  (Binary, $v: expr) => { (&$v[..]).into() };
  (Null, $v: expr) => { $v.into() }; // must check later
  (Unknown, $v: expr) => { $v.into() }; // must check later
  ($v: expr) => { $v.into() } // must check later
}
// pub to_sl3;

/// for tester
/// - create table tbl (
/// -   id integer primary key autoincrement,
/// -   b varchar(1),
/// -   u blob,
/// -   s text(16),
/// -   i integer,
/// -   f float
/// - );
/// - insert into tbl (b, u, s, i, f) values ("T", x'ee55', 'abc', -1, 2.0);
/// - insert into tbl (b, u, s, i, f) values ("F", x'2324', 'XYZ', 2, -1.0);
/// - select typeof(b), typeof(u), typeof(s), typeof(i), typeof(f) from tbl;
/// - text|blob|text|integer|real
#[struct_derive((id, b, u, s, i, f),
  (UInt64, Boolean, Binary, Utf8, Int8, Float32))]
#[tuple_sqlite3(UInt64, Boolean, Binary, Utf8, Int8, Float32)]
#[tuple_derive(UInt64, Boolean, Binary, Utf8, Int8, Float32)]
pub type Tester<'a> = (u64, bool, Vec<u8>, &'a str, i8, f32);

/// tester
/// - select * from tbl;
pub fn tester(dbn: &str) -> Result<(), Box<dyn Error>> {
  let n = StTester::members();
  let t = StTester::types();
  let qry = "select * from tbl where id > :id;";
  let p = vec![(":id", 0.into())];
/**/
  let df = df_from_sl3(dbn, &n, qry, &p,
    |row| StTester::from(row).into_vec());
  let mut df = df.expect("read DataFrame");
  tester_sub(&n, &mut df, "via StTester")?;
/**/
/**/
  let df = df_from_sl3_type(dbn, &n, &t, qry, &p,
    |row| StTester::from(row).into_vec());
  let mut df = df.expect("read DataFrame");
  tester_sub(&n, &mut df, "via StTester with names and types")?;
/**/
/**/
  let df = df_from_sl3(dbn, &n, qry, &p,
    |row| RecTester::from(row).into_iter().collect());
  let mut df = df.expect("read DataFrame");
  tester_sub(&n, &mut df, "via RecTester")?;
/**/
/**/
  let df = df_from_sl3_type(dbn, &n, &t, qry, &p,
    |row| RecTester::from(row).into_iter().collect());
  let mut df = df.expect("read DataFrame");
  tester_sub(&n, &mut df, "via RecTester with names and types")?;
/**/
  Ok(())
}

/// tester sub
pub fn tester_sub(n: &Vec<&str>, df: &mut DataFrame, inf: &str) ->
  Result<(), Box<dyn Error>> {
  println!("tester: {}", inf);

  let sc = named_schema(&df, n.to_vec());
  println!("{:?}", sc);
  println!("{}", df);

  let columns = df.get_columns();
  let column_3 = columns[3].utf8().expect("s as str"); // ChunkedArray
  let series_s = Series::from(column_3.apply_with_idx(|(i, s)|
    // std::borrow::Cow::Borrowed(s) // through
    format!("{}{:?}", s, (0..2).into_iter().map(|j|
      i + j).collect_tuple::<(usize, usize)>().expect("tuple")).into()
  ));
  df.replace("s", series_s).expect("replace df is_ok");
  println!("{}", df);

  Ok(())
}

/// tests
#[cfg(test)]
mod tests {
  use super::*;

  /// [-- --nocapture] [-- --show-output]
  #[test]
  fn test_polars_sqlite() {
    assert_eq!(RecTester::types(), StTester::types());
    assert_eq!(tester("./res/test_sqlite3_read.sl3").expect("tester"), ());
  }
}