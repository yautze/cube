use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt::Display};

pub const COMPARE_DATE_RANGE_FIELD: &str = "compareDateRange";
pub const COMPARE_DATE_RANGE_SEPARATOR: &str = " - ";
pub const BLENDING_QUERY_KEY_PREFIX: &str = "time.";
pub const BLENDING_QUERY_RES_SEPARATOR: &str = ".";
pub const MEMBER_SEPARATOR: &str = ".";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DBResponsePrimitive {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
pub enum DBResponseValue {
    DateTime(DateTime<Utc>),
    Primitive(DBResponsePrimitive),
    // TODO: Is this variant still used?
    Object { value: DBResponsePrimitive },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "compact")]
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    #[serde(rename = "regularQuery")]
    RegularQuery,
    #[serde(rename = "compareDateRangeQuery")]
    CompareDateRangeQuery,
    #[serde(rename = "blendingQuery")]
    BlendingQuery,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = serde_json::to_value(self)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        write!(f, "{}", str)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MemberType {
    #[serde(rename = "measures")]
    Measures,
    #[serde(rename = "dimensions")]
    Dimensions,
    #[serde(rename = "segments")]
    Segments,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    #[serde(rename = "equals")]
    Equals,
    #[serde(rename = "notEquals")]
    NotEquals,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "notContains")]
    NotContains,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "notIn")]
    NotIn,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "notSet")]
    NotSet,
    #[serde(rename = "inDateRange")]
    InDateRange,
    #[serde(rename = "notInDateRange")]
    NotInDateRange,
    #[serde(rename = "onTheDate")]
    OnTheDate,
    #[serde(rename = "beforeDate")]
    BeforeDate,
    #[serde(rename = "beforeOrOnDate")]
    BeforeOrOnDate,
    #[serde(rename = "afterDate")]
    AfterDate,
    #[serde(rename = "afterOrOnDate")]
    AfterOrOnDate,
    #[serde(rename = "measureFilter")]
    MeasureFilter,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryFilter {
    pub member: String,
    pub operator: FilterOperator,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct GroupingSet {
    pub group_type: String,
    pub id: u32,
    pub sub_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ParsedMemberExpression {
    pub expression: Vec<String>,
    #[serde(rename = "cubeName")]
    pub cube_name: String,
    pub name: String,
    #[serde(rename = "expressionName")]
    pub expression_name: String,
    pub definition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "groupingSet")]
    pub grouping_set: Option<GroupingSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTimeDimension {
    pub dimension: String,
    pub date_range: Option<Vec<String>>,
    pub compare_date_range: Option<Vec<String>>,
    pub granularity: Option<String>,
}

pub type AliasToMemberMap = HashMap<String, String>;

pub type MembersMap = HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranularityMeta {
    pub name: String,
    pub title: String,
    pub interval: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    pub title: String,
    pub short_title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub member_type: String,
    pub format: String,
    pub meta: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_members: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_members_grouped: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularities: Option<Vec<GranularityMeta>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub desc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedQueryFilter {
    pub member: String,
    pub operator: FilterOperator,
    pub values: Option<Vec<String>>,
    pub dimension: Option<String>,
}

// XXX: Omitted function variant
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum MemberOrMemberExpression {
    Member(String),
    MemberExpression(ParsedMemberExpression),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogicalAndFilter {
    pub and: Vec<LogicalFilter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogicalOrFilter {
    pub or: Vec<LogicalFilter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QueryFilterOrLogicalFilter {
    QueryFilter(QueryFilter),
    LogicalAndFilter(LogicalAndFilter),
    LogicalOrFilter(LogicalOrFilter),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogicalFilter {
    QueryFilter(QueryFilter),
    LogicalAndFilter(LogicalAndFilter),
    LogicalOrFilter(LogicalOrFilter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub measures: Vec<MemberOrMemberExpression>,
    pub dimensions: Option<Vec<MemberOrMemberExpression>>,
    pub filters: Option<Vec<LogicalFilter>>,
    #[serde(rename = "timeDimensions")]
    pub time_dimensions: Option<Vec<QueryTimeDimension>>,
    pub segments: Option<Vec<MemberOrMemberExpression>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub total: Option<bool>,
    #[serde(rename = "totalQuery")]
    pub total_query: Option<bool>,
    pub order: Option<Value>,
    pub timezone: Option<String>,
    #[serde(rename = "renewQuery")]
    pub renew_query: Option<bool>,
    pub ungrouped: Option<bool>,
    #[serde(rename = "responseFormat")]
    pub response_format: Option<ResultType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedQuery {
    pub measures: Vec<MemberOrMemberExpression>,
    pub dimensions: Option<Vec<MemberOrMemberExpression>>,
    #[serde(rename = "timeDimensions")]
    pub time_dimensions: Option<Vec<QueryTimeDimension>>,
    pub segments: Option<Vec<MemberOrMemberExpression>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub total: Option<bool>,
    #[serde(rename = "totalQuery")]
    pub total_query: Option<bool>,
    pub timezone: Option<String>,
    #[serde(rename = "renewQuery")]
    pub renew_query: Option<bool>,
    pub ungrouped: Option<bool>,
    #[serde(rename = "responseFormat")]
    pub response_format: Option<ResultType>,
    pub filters: Option<Vec<NormalizedQueryFilter>>,
    #[serde(rename = "rowLimit")]
    pub row_limit: Option<u32>,
    pub order: Option<Vec<Order>>,
    #[serde(rename = "queryType")]
    pub query_type: QueryType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransformedData {
    Compact {
        members: Vec<String>,
        dataset: Vec<Vec<DBResponsePrimitive>>,
    },
    Vanilla(Vec<HashMap<String, DBResponsePrimitive>>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransformDataRequest {
    #[serde(rename = "aliasToMemberNameMap")]
    pub alias_to_member_name_map: HashMap<String, String>,
    pub annotation: HashMap<String, ConfigItem>,
    pub data: Vec<HashMap<String, DBResponseValue>>,
    pub query: NormalizedQuery,
    #[serde(rename = "queryType")]
    pub query_type: QueryType,
    #[serde(rename = "resType")]
    pub res_type: Option<ResultType>,
}
