use crate::queryplanner::coalesce::SUPPORTED_COALESCE_TYPES;
use crate::queryplanner::hll::{Hll, HllUnion};
use crate::CubeError;
use chrono::{Datelike, Duration, Months, NaiveDateTime, TimeZone, Utc};
use datafusion::arrow::array::{
    Array, ArrayRef, BinaryArray, TimestampNanosecondArray, UInt64Builder,
};
use datafusion::arrow::datatypes::{DataType, IntervalUnit, TimeUnit};
use tokio_tungstenite::tungstenite::protocol::frame::coding::Data;
use std::any::Any;
// use datafusion::cube_ext::datetime::{date_addsub_array, date_addsub_scalar};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::function::AccumulatorArgs;
use datafusion::logical_expr::simplify::{ExprSimplifyResult, SimplifyInfo};
use datafusion::logical_expr::{
    AggregateUDF, AggregateUDFImpl, Expr, ScalarUDF, ScalarUDFImpl, Signature, TypeSignature, Volatility
};
use datafusion::physical_plan::{Accumulator, ColumnarValue};
use datafusion::scalar::ScalarValue;
use serde_derive::{Deserialize, Serialize};
use smallvec::smallvec;
use smallvec::SmallVec;
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CubeScalarUDFKind {
    HllCardinality, // cardinality(), accepting the HyperLogLog sketches.
    // Coalesce,
    // Now,
    UnixTimestamp,
    DateAdd,
    DateSub,
    DateBin,
}

pub fn scalar_udf_by_kind(k: CubeScalarUDFKind) -> Arc<ScalarUDF> {
    match k {
        CubeScalarUDFKind::HllCardinality => Arc::new(HllCardinality::descriptor()),
        // CubeScalarUDFKind::Coalesce => Box::new(Coalesce {}),
        // CubeScalarUDFKind::Now => Box::new(Now {}),
        CubeScalarUDFKind::UnixTimestamp => {
            Arc::new(ScalarUDF::new_from_impl(UnixTimestamp::new()))
        }
        CubeScalarUDFKind::DateAdd => todo!(), // Box::new(DateAddSub { is_add: true }),
        CubeScalarUDFKind::DateSub => todo!(), // Box::new(DateAddSub { is_add: false }),
        CubeScalarUDFKind::DateBin => todo!(), // Box::new(DateBin {}),
    }
}

/// Note that only full match counts. Pass capitalized names.
pub fn scalar_kind_by_name(n: &str) -> Option<CubeScalarUDFKind> {
    if n == "CARDINALITY" {
        return Some(CubeScalarUDFKind::HllCardinality);
    }
    // if n == "COALESCE" {
    //     return Some(CubeScalarUDFKind::Coalesce);
    // }
    // if n == "NOW" {
    //     return Some(CubeScalarUDFKind::Now);
    // }
    if n == "UNIX_TIMESTAMP" {
        return Some(CubeScalarUDFKind::UnixTimestamp);
    }
    if n == "DATE_ADD" {
        return Some(CubeScalarUDFKind::DateAdd);
    }
    if n == "DATE_SUB" {
        return Some(CubeScalarUDFKind::DateSub);
    }
    if n == "DATE_BIN" {
        return Some(CubeScalarUDFKind::DateBin);
    }
    return None;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CubeAggregateUDFKind {
    MergeHll, // merge(), accepting the HyperLogLog sketches.
}

pub trait CubeAggregateUDF {
    fn kind(&self) -> CubeAggregateUDFKind;
    fn name(&self) -> &str;
    fn descriptor(&self) -> AggregateUDF;
    fn accumulator(&self) -> Box<dyn Accumulator>;
}

pub fn aggregate_udf_by_kind(k: CubeAggregateUDFKind) -> Arc<AggregateUDF> {
    todo!();
    // match k {
    //     CubeAggregateUDFKind::MergeHll => Arc::new(AggregateUDF::new_from_impl(HllMergeUDF {})),
    // }
}

/// Note that only full match counts. Pass capitalized names.
pub fn aggregate_kind_by_name(n: &str) -> Option<CubeAggregateUDFKind> {
    if n == "MERGE" {
        return Some(CubeAggregateUDFKind::MergeHll);
    }
    return None;
}



// The rest of the file are implementations of the various functions that we have.
// TODO: add custom type and use it instead of `Binary` for HLL columns.

// TODO upgrade DF - remove?
// struct Coalesce {}
// impl Coalesce {
//     fn signature() -> Signature {
//         Signature::Variadic(SUPPORTED_COALESCE_TYPES.to_vec())
//     }
// }
// impl CubeScalarUDF for Coalesce {
//     fn kind(&self) -> CubeScalarUDFKind {
//         CubeScalarUDFKind::Coalesce
//     }
//
//     fn name(&self) -> &str {
//         "COALESCE"
//     }
//
//     fn descriptor(&self) -> ScalarUDF {
//         return ScalarUDF {
//             name: self.name().to_string(),
//             signature: Self::signature(),
//             return_type: Arc::new(|inputs| {
//                 if inputs.is_empty() {
//                     return Err(DataFusionError::Plan(
//                         "COALESCE requires at least 1 argument".to_string(),
//                     ));
//                 }
//                 let ts = type_coercion::data_types(inputs, &Self::signature())?;
//                 Ok(Arc::new(ts[0].clone()))
//             }),
//             fun: Arc::new(coalesce),
//         };
//     }
// }

// TODO upgrade DF - remove?
// struct Now {}
// impl Now {
//     fn signature() -> Signature {
//         Signature::Exact(Vec::new())
//     }
// }
// impl CubeScalarUDF for Now {
//     fn kind(&self) -> CubeScalarUDFKind {
//         CubeScalarUDFKind::Now
//     }
//
//     fn name(&self) -> &str {
//         "NOW"
//     }
//
//     fn descriptor(&self) -> ScalarUDF {
//         return ScalarUDF {
//             name: self.name().to_string(),
//             signature: Self::signature(),
//             return_type: Arc::new(|inputs| {
//                 assert!(inputs.is_empty());
//                 Ok(Arc::new(DataType::Timestamp(TimeUnit::Nanosecond, None)))
//             }),
//             fun: Arc::new(|_| {
//                 Err(DataFusionError::Internal(
//                     "NOW() was not optimized away".to_string(),
//                 ))
//             }),
//         };
//     }
// }

#[derive(Debug)]
struct UnixTimestamp {
    signature: Signature,
}

impl UnixTimestamp {
    pub fn new() -> Self {
        UnixTimestamp {
            signature: Self::signature(),
        }
    }
    fn signature() -> Signature {
        Signature::exact(Vec::new(), Volatility::Stable)
    }
}

impl ScalarUDFImpl for UnixTimestamp {
    fn name(&self) -> &str {
        "UNIX_TIMESTAMP"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> datafusion::common::Result<DataType> {
        Ok(DataType::Int64)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> datafusion::common::Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "UNIX_TIMESTAMP() was not optimized away".to_string(),
        ))
    }

    fn invoke_no_args(&self, _number_rows: usize) -> datafusion::common::Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "UNIX_TIMESTAMP() was not optimized away".to_string(),
        ))
    }

    fn simplify(
        &self,
        _args: Vec<Expr>,
        info: &dyn SimplifyInfo,
    ) -> datafusion::common::Result<ExprSimplifyResult> {
        let unix_time = info
            .execution_props()
            .query_execution_start_time
            .timestamp();
        Ok(ExprSimplifyResult::Simplified(Expr::Literal(
            ScalarValue::Int64(Some(unix_time)),
        )))
    }
}
//
// fn interval_dt_duration(i: &i64) -> Duration {
//     let days: i64 = i.signum() * (i.abs() >> 32);
//     let millis: i64 = i.signum() * ((i.abs() << 32) >> 32);
//     let duration = Duration::days(days) + Duration::milliseconds(millis);
//
//     duration
// }
//
// fn calc_intervals(start: NaiveDateTime, end: NaiveDateTime, interval: i32) -> i32 {
//     let years_diff = end.year() - start.year();
//     let months_diff = end.month() as i32 - start.month() as i32;
//     let mut total_months = years_diff * 12 + months_diff;
//
//     if total_months > 0 && end.day() < start.day() {
//         total_months -= 1; // If the day in the final date is less, reduce by 1 month
//     }
//
//     let rem = months_diff % interval;
//     let mut num_intervals = total_months / interval;
//
//     if num_intervals < 0 && rem == 0 && end.day() < start.day() {
//         num_intervals -= 1;
//     }
//
//     num_intervals
// }
//
// /// Calculate date_bin timestamp for source date for year-month interval
// fn calc_bin_timestamp_ym(origin: NaiveDateTime, source: &i64, interval: i32) -> NaiveDateTime {
//     let timestamp =
//         NaiveDateTime::from_timestamp(*source / 1_000_000_000, (*source % 1_000_000_000) as u32);
//     let num_intervals = calc_intervals(origin, timestamp, interval);
//     let nearest_date = if num_intervals >= 0 {
//         origin
//             .date()
//             .checked_add_months(Months::new((num_intervals * interval) as u32))
//             .unwrap_or(origin.date())
//     } else {
//         origin
//             .date()
//             .checked_sub_months(Months::new((-num_intervals * interval) as u32))
//             .unwrap_or(origin.date())
//     };
//
//     NaiveDateTime::new(nearest_date, origin.time())
// }
//
// /// Calculate date_bin timestamp for source date for date-time interval
// fn calc_bin_timestamp_dt(origin: NaiveDateTime, source: &i64, interval: &i64) -> NaiveDateTime {
//     let timestamp =
//         NaiveDateTime::from_timestamp(*source / 1_000_000_000, (*source % 1_000_000_000) as u32);
//     let diff = timestamp - origin;
//     let interval_duration = interval_dt_duration(&interval);
//     let num_intervals =
//         diff.num_nanoseconds().unwrap_or(0) / interval_duration.num_nanoseconds().unwrap_or(1);
//     let mut nearest_timestamp = origin
//         .checked_add_signed(interval_duration * num_intervals as i32)
//         .unwrap_or(origin);
//
//     if diff.num_nanoseconds().unwrap_or(0) < 0 {
//         nearest_timestamp = nearest_timestamp
//             .checked_sub_signed(interval_duration)
//             .unwrap_or(origin);
//     }
//
//     nearest_timestamp
// }
//
// struct DateBin {}
// impl DateBin {
//     fn signature() -> Signature {
//         Signature::OneOf(vec![
//             Signature::Exact(vec![
//                 DataType::Interval(IntervalUnit::YearMonth),
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//             ]),
//             Signature::Exact(vec![
//                 DataType::Interval(IntervalUnit::DayTime),
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//             ]),
//         ])
//     }
// }
// impl CubeScalarUDF for DateBin {
//     fn kind(&self) -> CubeScalarUDFKind {
//         CubeScalarUDFKind::DateBin
//     }
//
//     fn name(&self) -> &str {
//         "DATE_BIN"
//     }
//
//     fn descriptor(&self) -> ScalarUDF {
//         return ScalarUDF {
//             name: self.name().to_string(),
//             signature: Self::signature(),
//             return_type: Arc::new(|_| {
//                 Ok(Arc::new(DataType::Timestamp(TimeUnit::Nanosecond, None)))
//             }),
//             fun: Arc::new(move |inputs| {
//                 assert_eq!(inputs.len(), 3);
//                 let interval = match &inputs[0] {
//                     ColumnarValue::Scalar(i) => i.clone(),
//                     _ => {
//                         // We leave this case out for simplicity.
//                         // CubeStore does not allow intervals inside tables, so this is super rare.
//                         return Err(DataFusionError::Execution(format!(
//                             "Only scalar intervals are supported in DATE_BIN"
//                         )));
//                     }
//                 };
//
//                 let origin = match &inputs[2] {
//                     ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(Some(o))) => {
//                         NaiveDateTime::from_timestamp(
//                             *o / 1_000_000_000,
//                             (*o % 1_000_000_000) as u32,
//                         )
//                     }
//                     ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)) => {
//                         return Err(DataFusionError::Execution(format!(
//                             "Third argument (origin) of DATE_BIN must be a non-null timestamp"
//                         )));
//                     }
//                     _ => {
//                         // Leaving out other rare cases.
//                         // The initial need for the date_bin comes from custom granularities support
//                         // and there will always be a scalar origin point
//                         return Err(DataFusionError::Execution(format!(
//                             "Only scalar origins are supported in DATE_BIN"
//                         )));
//                     }
//                 };
//
//                 match interval {
//                     ScalarValue::IntervalYearMonth(Some(interval)) => match &inputs[1] {
//                         ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)) => Ok(
//                             ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)),
//                         ),
//                         ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(Some(t))) => {
//                             let nearest_timestamp = calc_bin_timestamp_ym(origin, t, interval);
//
//                             Ok(ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(
//                                 Some(nearest_timestamp.timestamp_nanos()),
//                             )))
//                         }
//                         ColumnarValue::Array(arr)
//                             if arr.as_any().is::<TimestampNanosecondArray>() =>
//                         {
//                             let ts_array = arr
//                                 .as_any()
//                                 .downcast_ref::<TimestampNanosecondArray>()
//                                 .unwrap();
//
//                             let mut builder = TimestampNanosecondArray::builder(ts_array.len());
//
//                             for i in 0..ts_array.len() {
//                                 if ts_array.is_null(i) {
//                                     builder.append_null()?;
//                                 } else {
//                                     let ts = ts_array.value(i);
//                                     let nearest_timestamp =
//                                         calc_bin_timestamp_ym(origin, &ts, interval);
//                                     builder.append_value(nearest_timestamp.timestamp_nanos())?;
//                                 }
//                             }
//
//                             Ok(ColumnarValue::Array(Arc::new(builder.finish()) as ArrayRef))
//                         }
//                         _ => {
//                             return Err(DataFusionError::Execution(format!(
//                                 "Second argument of DATE_BIN must be a non-null timestamp"
//                             )));
//                         }
//                     },
//                     ScalarValue::IntervalDayTime(Some(interval)) => match &inputs[1] {
//                         ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)) => Ok(
//                             ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)),
//                         ),
//                         ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(Some(t))) => {
//                             let nearest_timestamp = calc_bin_timestamp_dt(origin, t, &interval);
//
//                             Ok(ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(
//                                 Some(nearest_timestamp.timestamp_nanos()),
//                             )))
//                         }
//                         ColumnarValue::Array(arr)
//                             if arr.as_any().is::<TimestampNanosecondArray>() =>
//                         {
//                             let ts_array = arr
//                                 .as_any()
//                                 .downcast_ref::<TimestampNanosecondArray>()
//                                 .unwrap();
//
//                             let mut builder = TimestampNanosecondArray::builder(ts_array.len());
//
//                             for i in 0..ts_array.len() {
//                                 if ts_array.is_null(i) {
//                                     builder.append_null()?;
//                                 } else {
//                                     let ts = ts_array.value(i);
//                                     let nearest_timestamp =
//                                         calc_bin_timestamp_dt(origin, &ts, &interval);
//                                     builder.append_value(nearest_timestamp.timestamp_nanos())?;
//                                 }
//                             }
//
//                             Ok(ColumnarValue::Array(Arc::new(builder.finish()) as ArrayRef))
//                         }
//                         _ => {
//                             return Err(DataFusionError::Execution(format!(
//                                 "Second argument of DATE_BIN must be a non-null timestamp"
//                             )));
//                         }
//                     },
//                     _ => Err(DataFusionError::Execution(format!(
//                         "Unsupported interval type: {:?}",
//                         interval
//                     ))),
//                 }
//             }),
//         };
//     }
// }
//
// struct DateAddSub {
//     is_add: bool,
// }
//
// impl DateAddSub {
//     fn signature() -> Signature {
//         Signature::OneOf(vec![
//             Signature::Exact(vec![
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//                 DataType::Interval(IntervalUnit::YearMonth),
//             ]),
//             Signature::Exact(vec![
//                 DataType::Timestamp(TimeUnit::Nanosecond, None),
//                 DataType::Interval(IntervalUnit::DayTime),
//             ]),
//         ])
//     }
// }
//
// impl DateAddSub {
//     fn name_static(&self) -> &'static str {
//         match self.is_add {
//             true => "DATE_ADD",
//             false => "DATE_SUB",
//         }
//     }
// }
//
// impl CubeScalarUDF for DateAddSub {
//     fn kind(&self) -> CubeScalarUDFKind {
//         match self.is_add {
//             true => CubeScalarUDFKind::DateAdd,
//             false => CubeScalarUDFKind::DateSub,
//         }
//     }
//
//     fn name(&self) -> &str {
//         self.name_static()
//     }
//
//     fn descriptor(&self) -> ScalarUDF {
//         let name = self.name_static();
//         let is_add = self.is_add;
//         return ScalarUDF {
//             name: self.name().to_string(),
//             signature: Self::signature(),
//             return_type: Arc::new(|_| {
//                 Ok(Arc::new(DataType::Timestamp(TimeUnit::Nanosecond, None)))
//             }),
//             fun: Arc::new(move |inputs| {
//                 assert_eq!(inputs.len(), 2);
//                 let interval = match &inputs[1] {
//                     ColumnarValue::Scalar(i) => i.clone(),
//                     _ => {
//                         // We leave this case out for simplicity.
//                         // CubeStore does not allow intervals inside tables, so this is super rare.
//                         return Err(DataFusionError::Execution(format!(
//                             "Only scalar intervals are supported in `{}`",
//                             name
//                         )));
//                     }
//                 };
//                 match &inputs[0] {
//                     ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)) => Ok(
//                         ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(None)),
//                     ),
//                     ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(Some(t))) => {
//                         let r = date_addsub_scalar(Utc.timestamp_nanos(*t), interval, is_add)?;
//                         Ok(ColumnarValue::Scalar(ScalarValue::TimestampNanosecond(
//                             Some(r.timestamp_nanos()),
//                         )))
//                     }
//                     ColumnarValue::Array(t) if t.as_any().is::<TimestampNanosecondArray>() => {
//                         let t = t
//                             .as_any()
//                             .downcast_ref::<TimestampNanosecondArray>()
//                             .unwrap();
//                         Ok(ColumnarValue::Array(Arc::new(date_addsub_array(
//                             &t, interval, is_add,
//                         )?)))
//                     }
//                     _ => {
//                         return Err(DataFusionError::Execution(format!(
//                             "First argument of `{}` must be a non-null timestamp",
//                             name
//                         )))
//                     }
//                 }
//             }),
//         };
//     }
// }
//

#[derive(Debug)]
struct HllCardinality {
    signature: Signature,
}
impl HllCardinality {
    pub fn new() -> HllCardinality {
        // TODO upgrade DF: Is it Volatile or Immutable?
        let signature = Signature::new(TypeSignature::Exact(vec![DataType::Binary]), Volatility::Volatile);

        HllCardinality{
            signature
        }
    }
    fn descriptor() -> ScalarUDF {
        return ScalarUDF::new_from_impl(HllCardinality::new());
    }
}

impl ScalarUDFImpl for HllCardinality {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn name(&self) -> &str {
        "CARDINALITY"
    }
    fn signature(&self) -> &Signature {
        &self.signature
    }
    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType, DataFusionError> {
        Ok(DataType::UInt64)
    }
    fn invoke(&self, args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
        assert_eq!(args.len(), 1);
        let sketches = args[0].clone().into_array(1)?;
        let sketches = sketches
            .as_any()
            .downcast_ref::<BinaryArray>()
            .expect("expected binary data");

        let mut r = UInt64Builder::with_capacity(sketches.len());
        for s in sketches {
            match s {
                None => r.append_null(),
                Some(d) => {
                    if d.len() == 0 {
                        r.append_value(0)
                    } else {
                        r.append_value(read_sketch(d)?.cardinality())
                    }
                }
            }
        }
        return Ok(ColumnarValue::Array(Arc::new(r.finish())));
    }
    fn aliases(&self) -> &[String] {
        &[]
    }
}

//
// #[derive(Debug)]
// struct HllMergeUDF {}
// impl AggregateUDFImpl for HllMergeUDF {
//
//     fn name(&self) -> &str {
//         return "MERGE";
//     }
//
//     fn as_any(&self) -> &dyn Any {
//         &self
//     }
//
//     fn signature(&self) -> &Signature {
//         &Signature::exact(vec![DataType::Binary], Volatility::Stable)
//     }
//
//     fn return_type(&self, arg_types: &[DataType]) -> datafusion::common::Result<DataType> {
//         Ok(DataType::Binary)
//     }
//
//     fn accumulator(&self, acc_args: AccumulatorArgs) -> datafusion::common::Result<Box<dyn Accumulator>> {
//         Ok(Box::new(HllMergeAccumulator { acc: None }))
//     }
// }
//
// #[derive(Debug)]
// struct HllMergeAccumulator {
//     // TODO: store sketch for empty set from the start.
//     //       this requires storing index_bit_len in the type.
//     acc: Option<HllUnion>,
// }
//
// impl Accumulator for HllMergeAccumulator {
//     fn reset(&mut self) {
//         self.acc = None;
//     }
//
//     fn state(&self) -> Result<SmallVec<[ScalarValue; 2]>, DataFusionError> {
//         return Ok(smallvec![self.evaluate()?]);
//     }
//
//     fn update(&mut self, row: &[ScalarValue]) -> Result<(), DataFusionError> {
//         assert_eq!(row.len(), 1);
//         let data;
//         if let ScalarValue::Binary(v) = &row[0] {
//             if let Some(d) = v {
//                 data = d
//             } else {
//                 return Ok(()); // ignore NULL.
//             }
//         } else {
//             return Err(CubeError::internal(
//                 "invalid scalar value passed to MERGE, expecting HLL sketch".to_string(),
//             )
//             .into());
//         }
//
//         // empty state is ok, this means an empty sketch.
//         if data.len() == 0 {
//             return Ok(());
//         }
//         return self.merge_sketch(read_sketch(&data)?);
//     }
//
//     fn merge(&mut self, states: &[ScalarValue]) -> Result<(), DataFusionError> {
//         assert_eq!(states.len(), 1);
//
//         let data;
//         if let ScalarValue::Binary(v) = &states[0] {
//             if let Some(d) = v {
//                 data = d
//             } else {
//                 return Ok(()); // ignore NULL.
//             }
//         } else {
//             return Err(CubeError::internal("invalid state in MERGE".to_string()).into());
//         }
//         // empty state is ok, this means an empty sketch.
//         if data.len() == 0 {
//             return Ok(());
//         }
//         return self.merge_sketch(read_sketch(&data)?);
//     }
//
//     fn evaluate(&self) -> Result<ScalarValue, DataFusionError> {
//         let v;
//         match &self.acc {
//             None => v = Vec::new(),
//             Some(s) => v = s.write(),
//         }
//         return Ok(ScalarValue::Binary(Some(v)));
//     }
// }
//
// impl HllMergeAccumulator {
//     fn merge_sketch(&mut self, s: Hll) -> Result<(), DataFusionError> {
//         if self.acc.is_none() {
//             self.acc = Some(HllUnion::new(s)?);
//             return Ok(());
//         } else if let Some(acc_s) = &mut self.acc {
//             if !acc_s.is_compatible(&s) {
//                 return Err(CubeError::internal(
//                     "cannot merge two incompatible HLL sketches".to_string(),
//                 )
//                 .into());
//             }
//             acc_s.merge_with(s)?;
//         } else {
//             unreachable!("impossible");
//         }
//         return Ok(());
//     }
// }

pub fn read_sketch(data: &[u8]) -> Result<Hll, DataFusionError> {
    return Hll::read(&data).map_err(|e| DataFusionError::Execution(e.message));
}
