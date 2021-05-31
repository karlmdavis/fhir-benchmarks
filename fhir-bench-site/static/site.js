/*
 * Add click events to the navbar's burger menu (which appears at mobile widths).
 *
 * Reference: <https://bulma.io/documentation/components/navbar/#navbar-burger>.
 */
document.addEventListener('DOMContentLoaded', () => {
  // Get all "navbar-burger" elements
  const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);

  // Check if there are any navbar burgers
  if ($navbarBurgers.length > 0) {

    // Add a click event on each of them
    $navbarBurgers.forEach( el => {
      el.addEventListener('click', () => {
        // Get the target from the "data-target" attribute
        const target = el.dataset.target;
        const $target = document.getElementById(target);

        // Toggle the "is-active" class on both the "navbar-burger" and the "navbar-menu"
        el.classList.toggle('is-active');
        $target.classList.toggle('is-active');
      });
    });
  }
});

document.querySelectorAll('section.operation .tabs a').forEach((tabSwitcher) => {
  tabSwitcher.addEventListener('click', (e) => {
    let operation = e.currentTarget.dataset.operation;
    let concurrentUsers = e.currentTarget.dataset.concurrentUsers;

    // Switch out the table that is being shown.
    document.querySelectorAll(`section[data-operation='${operation}'] table.measurement`).forEach((measurementTable) => {
      if (concurrentUsers == measurementTable.dataset.concurrentUsers) {
        measurementTable.classList.remove("is-hidden");
      } else {
        measurementTable.classList.add("is-hidden");
      }
    });

    // Switch which tab is marked active.
    document.querySelectorAll(`section[data-operation='${operation}'] .tabs li`).forEach((tabSwitcherParent) => {
      if (concurrentUsers == tabSwitcherParent.dataset.concurrentUsers) {
        tabSwitcherParent.classList.add("is-active");
      } else {
        tabSwitcherParent.classList.remove("is-active");
      }
    });
  });
});

/*
 * Returns the throughput value for the specified measurement table row.
 *
 * Paremeters:
 * * `measurementTableRow`: the table row to get the throughput value from
 */
function getThroughputValue(measurementTableRow) {
  let throughputCell = measurementTableRow.cells[1];
  let throughputElement = throughputCell.querySelector('abbr');
  let throughputText = throughputElement.getAttribute('title');
  let throughputValue = parseFloat(throughputText);

  return throughputValue;
}

/*
 * Sorts the specified table's rows numerically by the values produced by the specified function.
 *
 * Paremeters:
 * * `tableElement`: the table who's rows should be sorted
 * * `sortValueExtractor`: a function that takes in a table row and returns the value to sort it by
 *
 * Reference: <https://stackoverflow.com/a/7558600>
 */
function sortTable(tableElement, sortValueExtractor) {
  let rowsParent = tableElement.tBodies[0];
  
  let store = [];
  for (let row of rowsParent.rows) {
    let sortValue = sortValueExtractor(row);
    if (!isNaN(sortValue)) {
      store.push([sortValue, row]);
    }
  }

  store.sort(function(x, y) {
    return y[0] - x[0];
  });

  for (let storeElement of store) {
    rowsParent.appendChild(storeElement[1]);
  }
}

/*
 * Returns a color "in between" the two specified colors.
 *
 * Parameters:
 * * `color_1`: the first color
 * * `color_2`: the second color
 * * `weight`: an integer value between 0 and 100 for how "far" to shift between the two colors: 0 for just
 *     `color_1`, 100 for just `color_2`
 *
 * Reference: <https://gist.github.com/jedfoster/7939513>
 */
function mix(color_1, color_2, weight) {
  function d2h(d) { return d.toString(16); }  // convert a decimal value to hex
  function h2d(h) { return parseInt(h, 16); } // convert a hex value to decimal 

  weight = (typeof(weight) !== 'undefined') ? weight : 50; // set the weight to 50%, if that argument is omitted

  var color = "#";

  for(var i = 0; i <= 5; i += 2) { // loop through each of the 3 hex pairs—red, green, and blue
    var v1 = h2d(color_1.substr(i, 2)), // extract the current pairs
        v2 = h2d(color_2.substr(i, 2)),
        
        // combine the current pairs from each source color, according to the specified weight
        val = d2h(Math.floor(v2 + (v1 - v2) * (weight / 100.0))); 

    while(val.length < 2) { val = '0' + val; } // prepend a '0' if val results in a single digit
    
    color += val; // concatenate val to our new color string
  }
    
  return color; // PROFIT!
};

document.querySelectorAll('table.measurement').forEach((measurementTable) => {
  // Sort each measurement table by throughput, descending.
  sortTable(measurementTable, getThroughputValue);

  // Calculate the "percentage of max throughput" values for each measurement and adjust the progress bar
  // and text elements accordingly.
  let maxThroughputValue = getThroughputValue(measurementTable.tBodies[0].rows[0]);
  measurementTable.querySelectorAll('tbody tr').forEach((row) => {
    let throughputValue = getThroughputValue(row);
    let throughputPercent = (throughputValue / maxThroughputValue) * 100;
    let throughputPercentBar = row.querySelector('.throughput_percent_bar');
    let throughputPercentText = row.querySelector('.throughput_percent_text');

    // Note: the first color here is Bulma's `$success` and the second is its `$danger`.
    throughputPercentBar.style.backgroundColor = mix('58cc98', 'f24e6f', Math.round(throughputPercent));
    
    throughputPercentBar.style.width = `calc((100% - 3.0em) * (${throughputPercent} / 100))`;
    throughputPercentText.textContent = Math.floor(throughputPercent) + '%';
  });
});

// I honestly have no clue what this really does, except that it's important.
function appendDataSeries(histo, name, dataSeries) {
  var series;
  var seriesCount;
  if (dataSeries.length == 0) {
    series = [['X', name]];
    seriesCount = 1;
  } else {
    series = dataSeries;
    series[0].push(name);
    seriesCount = series[0].length - 1;
  }

  var lines = histo.split("\n");

  var seriesIndex = 1;
  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trim();
    var values = line.trim().split(/[ ]+/);

    if (line[0] != '#' && values.length == 4) {
      var y = parseFloat(values[0]);
      var x = parseFloat(values[3]);

      if (!isNaN(x) && !isNaN(y)) {
        if (seriesIndex >= series.length) {
          series.push([x]);
        }

        while (series[seriesIndex].length < seriesCount) {
          series[seriesIndex].push(null);
        }

        series[seriesIndex].push(y);
        seriesIndex++;
      }
    }
  }

  while (seriesIndex < series.length) {
    series[seriesIndex].push(null);
    seriesIndex++;
  }

  return series;
}

// Converts histogram data in the 'HIST...' format to a Google DataTable.
function createDataTableForHistogram(histogramData) {
  if (!histogramData.startsWith('HIST')) {
    throw 'Invalid histogram data: ' + histogramData;
  }
  let histogram = hdr.decodeFromCompressedBase64(histogramData, 32, true);
  let histogramPercentilesText = histogram.outputPercentileDistribution();
  let dataSeries = appendDataSeries(histogramPercentilesText, 'A', []);
  
  return google.visualization.arrayToDataTable(dataSeries);
}

// Draws a chart for the specified histogram data in the specified element.
function drawChart(dataTable, targetElement) {
  let ticks =
    [{ v: 1, f: '0%' },
    { v: 10, f: '90%' },
    { v: 100, f: '99%' },
    { v: 1000, f: '99.9%' },
    { v: 10000, f: '99.99%' },
    { v: 100000, f: '99.999%' },
    { v: 1000000, f: '99.9999%' },
    { v: 10000000, f: '99.99999%' },
    { v: 100000000, f: '99.999999%' }];
  
  // Round up to the max ticks.v value needed, using base-10 logs.
  let maxPercentile = Math.pow(10, Math.ceil(Math.log10(dataTable.getColumnRange(0).max)));

  // The table rows alternate color, so grab the right background color.
  let backgroundColor = targetElement.style.backgroundColor;

  let options = {
    title: 'Latency by Percentile Distribution',
    height: 300,
    backgroundColor: backgroundColor,
    hAxis: {
      title: "Percentile",
      minValue: 1,
      logScale: true,
      ticks: ticks,
      viewWindowMode: 'explicit',
      viewWindow: {
        max: maxPercentile,
        min: 1
      }
    },
    vAxis: { title: 'Latency (milliseconds)', minValue: 0 },
    legend: { position: 'none' },
    fontName: 'Roboto',
    theme: 'maximized'
  };

  let chart = new google.visualization.LineChart(targetElement);

  // Add tooptips with correct percentile text to data.
  let columns = [0];
  for (var i = 1; i < dataTable.getNumberOfColumns(); i++) {
    columns.push(i);
    columns.push({
      type: 'string',
      properties: {
        role: 'tooltip'
      },
      calc: (function (j) {
        return function (dt, row) {
          var percentile = 100.0 - (100.0 / dt.getValue(row, 0));
          return percentile.toPrecision(7) +
              '\%\'ile = ' + dt.getValue(row, j) + ' milliseconds'
        }
      })(i)
    });
  }
  let view = new google.visualization.DataView(dataTable);
  view.setColumns(columns);

  chart.draw(view, options);
}

document.querySelectorAll('section.operation .latency_toggle').forEach((latencyToggle) => {
  latencyToggle.addEventListener('click', (e) => {
    let latencyToggleIcon = latencyToggle.querySelector('i');
    let latencyToggleCell = latencyToggle.parentElement;
    let latencyToggleRow = latencyToggleCell.parentElement;

    // The toggle will be fa-angle-down when inactive and fa-angle-up when active.
    let isActive = latencyToggleIcon.classList.contains("fa-angle-up");

    if (isActive) {
      latencyToggleIcon.classList.remove("fa-angle-up");
      latencyToggleIcon.classList.add("fa-angle-down");

      let latencyDisplayRow = latencyToggleRow.nextSibling;
      if (!latencyDisplayRow.classList.contains("latency_display")) {
        throw 'Unable to find latency display row.';
      }

      latencyDisplayRow.remove();
    } else {
      latencyToggleIcon.classList.remove("fa-angle-down");
      latencyToggleIcon.classList.add("fa-angle-up");

      let latencyDisplayRow = document.createElement("tr");
      latencyDisplayRow.classList.add("latency_display");
      let latencyDisplayCell = document.createElement("td");
      latencyDisplayCell.setAttribute("colspan", 5);
      latencyDisplayRow.appendChild(latencyDisplayCell);
      latencyToggleRow.parentElement.insertBefore(latencyDisplayRow, latencyToggleRow.nextSibling);

      // Set a callback to run when HdrHistogramJS WASM & the Google Visualization API are loaded.
      google.load('visualization', '1.0', { 'packages': ['corechart'] });
      hdr.initWebAssembly().then(
        () => google.setOnLoadCallback(() => {
          let histogramData = latencyToggle.dataset.latencyHistogram;
          let histogramDataTable = createDataTableForHistogram(histogramData);
          drawChart(histogramDataTable, latencyDisplayCell);
        })
      );
    }
  });
});