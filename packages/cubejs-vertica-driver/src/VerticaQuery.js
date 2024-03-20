const moment = require('moment-timezone');
const { BaseFilter, BaseQuery } = require('@cubejs-backend/schema-compiler');

const GRANULARITY_TO_INTERVAL = {
  week: (date) => `DATE_TRUNC('week', ${date})`,
  second: (date) => `DATE_TRUNC('second', ${date})`,
  minute: (date) => `DATE_TRUNC('minute', ${date})`,
  hour: (date) => `DATE_TRUNC('hour', ${date})`,
  day: (date) => `DATE_TRUNC('day', ${date})`,
  month: (date) => `DATE_TRUNC('month', ${date})`,
  quarter: (date) => `DATE_TRUNC('quarter', ${date})`,
  year: (date) => `DATE_TRUNC('year', ${date})`
};

class VerticaFilter extends BaseFilter {
  likeIgnoreCase(column, not, param, type) {
    const p = (!type || type === 'contains' || type === 'ends') ? '%' : '';
    const s = (!type || type === 'contains' || type === 'starts') ? '%' : '';
    return ` ILIKE (${column}${not ? ' NOT' : ''}, CONCAT('${p}', ${this.allocateParam(param)}, '${s}'))`;
  }

  castParameter() {
    if (this.definition().type === 'boolean') {
      return 'CAST(? AS BOOLEAN)';
    } else if (this.measure || this.definition().type === 'number') {
      return 'CAST(? AS DOUBLE)';
    }

    return '?';
  }
}

class VerticaQuery extends BaseQuery {
  newFilter(filter) {
    return new VerticaFilter(this, filter);
  }

  convertTz(field) {
    return `${field} AT TIMEZONE '${this.timezone}'`;
  }

  timeStampCast(value) {
    return `TO_TIMESTAMP(${value}, 'YYYY-MM-DD"T"HH24:MI:SS.FFF')`;
  }

  timestampFormat() {
    return moment.HTML5_FMT.DATETIME_LOCAL_MS;
  }

  dateTimeCast(value) {
    return `${value}::TIMESTAMP`;
  }

  timeGroupedColumn(granularity, dimension) {
    return GRANULARITY_TO_INTERVAL[granularity](dimension);
  }

  escapeColumnName(name) {
    return `"${name}"`;
  }

  seriesSql(timeDimension) {
    const values = timeDimension.timeSeries().map(
      ([from, to]) => `SELECT '${from}' f, '${to}' t`
    ).join(' UNION ALL ');

    return `SELECT dates.f::TIMESTAMP date_from, dates.t::TIMESTAMP date_to FROM (${values}) AS dates`;
  }

  concatStringsSql(strings) {
    return `CONCAT(${strings.join(', ')})`;
  }

  unixTimestampSql() {
    return 'EXTRACT(EPOCH FROM now())';
  }

  wrapSegmentForDimensionSelect(sql) {
    return `IF(${sql}, 1, 0)`;
  }
}

module.exports = VerticaQuery;
