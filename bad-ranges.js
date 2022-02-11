'use strict';

const moment = require('moment');

// There have been periods where results cannot be considered valid and
// contribute noise to the metrics. These date ranges are listed below, with
// inclusive start dates and exclusive end dates.

const STABLE_BAD_RANGES = [
  // This was some form of Safari outage, undiagnosed but a clear erroneous
  // spike in failure rates.
  [moment('2019-02-06'), moment('2019-03-04')],
  // This was a safaridriver outage, resolved by
  // https://github.com/web-platform-tests/wpt/pull/18585
  [moment('2019-06-27'), moment('2019-08-23')],
  // This was a general outage due to the Taskcluster Checks migration.
  [moment('2020-07-08'), moment('2020-07-16')],
  // This was a Firefox outage which produced only partial test results.
  [moment('2020-07-21'), moment('2020-08-15')],
  // This was a regression from https://github.com/web-platform-tests/wpt/pull/29089,
  // fixed by https://github.com/web-platform-tests/wpt/pull/32540
  [moment('2022-01-25'), moment('2022-01-27')],
];

const EXPERIMENTAL_BAD_RANGES = [
  // This was a safaridriver outage, resolved by
  // https://github.com/web-platform-tests/wpt/pull/18585
  [moment('2019-06-27'), moment('2019-08-23')],
  // This was a general outage due to the Taskcluster Checks migration.
  [moment('2020-07-08'), moment('2020-07-16')],
  // This was a regression from https://github.com/web-platform-tests/wpt/pull/29089,
  // fixed by https://github.com/web-platform-tests/wpt/pull/32540
  [moment('2022-01-25'), moment('2022-01-27')],
];

// Advances date to the end of a bad range if it's in a bad range, and otherwise
// returns the same date value.
function advanceDateToSkipBadDataIfNecessary(date, experimental) {
  const ranges = experimental ? EXPERIMENTAL_BAD_RANGES : STABLE_BAD_RANGES;
  for (const range of ranges) {
    if (date >= range[0] && date < range[1]) {
      console.log(`Skipping from ${date.format('YYYY-MM-DD')} to ` +
          `${range[1].format('YYYY-MM-DD')} due to bad data`);
      return range[1];
    }
  }
  return date;
}


module.exports = {advanceDateToSkipBadDataIfNecessary};