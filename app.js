// Default slider values. Reset uses this object.
const defaults = {
  promisedReturn: 15,
  dropout: 5,
  recruitmentDecay: 6,
  initialParticipants: 500,
  cycles: 48,
  populationCap: 5_000_000,
};

// Small metadata for display text next to each slider.
const controls = {
  promisedReturn: { suffix: "%" },
  dropout: { suffix: "%" },
  recruitmentDecay: { suffix: "%" },
  initialParticipants: { suffix: " participants" },
  cycles: { suffix: " cycles" },
  populationCap: { suffix: " people" },
};

// Number formatters so numbers are easy to read in UI.
const numberFmt = new Intl.NumberFormat();
const decimalFmt = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });

// Build all slider IDs from the controls object so we do not repeat ourselves.
const sliderIds = Object.keys(controls);

// Grab DOM nodes for each slider and its output label.
const sliders = Object.fromEntries(sliderIds.map((id) => [id, document.getElementById(id)]));
const outputs = Object.fromEntries(sliderIds.map((id) => [id, document.getElementById(`${id}Value`)]));

// Main UI elements we update during simulation.
const runBtn = document.getElementById("runBtn");
const resetBtn = document.getElementById("resetBtn");
const summary = document.getElementById("summary");
const resultsBody = document.getElementById("resultsBody");
const chartCanvas = document.getElementById("chart");

// Fixed educational message shown above chart/table.
const summaryText = `Everyone puts in 1 unit.
Old participants get paid using money from new participants.

There's no real growth or business happening so its a scam.

As recruitment slows and people drop out, the promises keep growing... but the new people don't.

Eventually, you need more newcomers than exist.

Boom. Collapse.`;

// Shortens big numbers for y-axis labels (for example 1200000 -> 1.2M).
function formatAxisNumber(value) {
  const abs = Math.abs(value);
  if (abs >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(1)}B`;
  }
  if (abs >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (abs >= 1_000) {
    return `${(value / 1_000).toFixed(1)}k`;
  }
  return numberFmt.format(Math.round(value));
}

// Formats slider readout values like "15%" or "500 participants".
function formatControlValue(id, value) {
  if (id === "promisedReturn" || id === "dropout" || id === "recruitmentDecay") {
    return `${value}%`;
  }
  return `${numberFmt.format(value)}${controls[id].suffix.replace(" people", "")}`;
}

// Formats table values with up to 2 decimals.
function formatParticipants(value) {
  return decimalFmt.format(value);
}

// Updates live text labels above sliders while user drags controls.
function setOutputText() {
  sliderIds.forEach((id) => {
    const value = Number(sliders[id].value);
    if (id === "populationCap") {
      outputs[id].textContent = `${numberFmt.format(value)} people`;
      return;
    }
    outputs[id].textContent = formatControlValue(id, value);
  });
}

// Puts all sliders back to default values.
function resetInputs() {
  sliderIds.forEach((id) => {
    sliders[id].value = defaults[id];
  });
  setOutputText();
}

// Reads current slider values and converts percentages to decimals.
function readInputs() {
  return {
    promisedReturn: Number(sliders.promisedReturn.value) / 100,
    dropout: Number(sliders.dropout.value) / 100,
    recruitmentDecay: Number(sliders.recruitmentDecay.value) / 100,
    initialParticipants: Number(sliders.initialParticipants.value),
    cycles: Number(sliders.cycles.value),
    populationCap: Number(sliders.populationCap.value),
  };
}

// Core simulation logic.
// This keeps everything intentionally simple and educational.
function simulate(params) {
  const rows = [];

  // We start with a base recruiting power per active participant.
  // This is not "real finance", just a simple model knob.
  const baseRecruitmentRate = 0.52;

  // Track current system state as we move cycle by cycle.
  let obligations = params.initialParticipants;
  let activeParticipants = params.initialParticipants;
  let cumulativeRequired = params.initialParticipants;
  let cumulativeActual = params.initialParticipants;

  // These get filled in once collapse is detected.
  let collapseCycle = null;
  let collapseReason = "";

  for (let cycle = 1; cycle <= params.cycles; cycle += 1) {
    // Required newcomers this cycle come from growing obligations.
    const requiredNew = obligations * params.promisedReturn;
    cumulativeRequired += requiredNew;

    // Dropout reduces current active participants first.
    activeParticipants *= 1 - params.dropout;

    // Recruiting power decays over time (cycle 1 has no decay yet).
    const decayFactor = Math.pow(1 - params.recruitmentDecay, cycle - 1);

    // Available new participants this cycle, before population cap check.
    const recruitmentPotential = activeParticipants * baseRecruitmentRate * decayFactor;

    // Remaining room before we hit population cap.
    const remainingPopulation = Math.max(0, params.populationCap - cumulativeActual);

    // Actual newcomers cannot exceed either recruitment potential or cap.
    const actualNew = Math.min(recruitmentPotential, remainingPopulation);

    // Collapse checks from the project requirements.
    const capExceeded = cumulativeRequired > params.populationCap;
    const recruitmentExceeded = requiredNew > actualNew;

    const status = capExceeded || recruitmentExceeded ? "COLLAPSED" : "OK";

    // Save one row for chart/table output.
    rows.push({
      cycle,
      activeParticipants,
      requiredNew,
      actualNew,
      status,
    });

    // Stop as soon as collapse happens.
    if (status === "COLLAPSED") {
      collapseCycle = cycle;
      collapseReason = capExceeded
        ? "cumulative required participants exceeded the population cap"
        : "required growth outpaced available recruitment";
      break;
    }

    // Move state into the next cycle.
    obligations = obligations * (1 + params.promisedReturn) + actualNew;
    activeParticipants += actualNew;
    cumulativeActual += actualNew;

    // Safety stop in case numbers become infinite.
    if (!Number.isFinite(obligations) || !Number.isFinite(activeParticipants)) {
      collapseCycle = cycle;
      collapseReason = "values exploded beyond realistic population scale";
      rows[rows.length - 1].status = "COLLAPSED";
      break;
    }
  }

  return { rows, collapseCycle, collapseReason };
}

// Draws required vs actual newcomers over cycles on a canvas.
function drawChart(rows) {
  const ctx = chartCanvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = chartCanvas.clientWidth;
  const cssHeight = Math.max(320, Math.round(cssWidth * 0.42));

  chartCanvas.width = Math.round(cssWidth * dpr);
  chartCanvas.height = Math.round(cssHeight * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  // Drawing area and chart padding.
  const width = cssWidth;
  const height = cssHeight;
  const pad = { top: 34, right: 16, bottom: 58, left: 108 };

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, width, height);

  // Figure out chart scale from max data value.
  const plotWidth = width - pad.left - pad.right;
  const plotHeight = height - pad.top - pad.bottom;
  const maxValue = Math.max(
    1,
    ...rows.map((row) => row.requiredNew),
    ...rows.map((row) => row.actualNew),
  );

  // Helpers to map data coordinates to canvas pixel coordinates.
  const x = (index) => {
    if (rows.length <= 1) {
      return pad.left + plotWidth / 2;
    }
    return pad.left + (index / (rows.length - 1)) * plotWidth;
  };
  const y = (value) => pad.top + (1 - value / maxValue) * plotHeight;

  // Horizontal grid lines + y-axis labels.
  ctx.strokeStyle = "#d9e4f3";
  ctx.lineWidth = 1;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let i = 0; i <= 5; i += 1) {
    const yy = pad.top + (i / 5) * plotHeight;
    ctx.beginPath();
    ctx.moveTo(pad.left, yy);
    ctx.lineTo(width - pad.right, yy);
    ctx.stroke();

    const labelValue = maxValue * (1 - i / 5);
    ctx.fillStyle = "#4d6685";
    ctx.font = "12px 'IBM Plex Mono', monospace";
    ctx.fillText(formatAxisNumber(labelValue), pad.left - 12, yy);
  }
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";

  // Build a small set of x ticks so labels stay readable.
  const desiredXTicks = Math.min(8, rows.length);
  const tickStep = Math.max(1, Math.ceil((rows.length - 1) / Math.max(1, desiredXTicks - 1)));
  const tickIndexes = [];
  for (let i = 0; i < rows.length; i += tickStep) {
    tickIndexes.push(i);
  }
  if (rows.length > 1 && tickIndexes[tickIndexes.length - 1] !== rows.length - 1) {
    tickIndexes.push(rows.length - 1);
  }

  // Vertical guide lines at x ticks.
  ctx.strokeStyle = "#edf3fb";
  tickIndexes.forEach((idx) => {
    const xx = x(idx);
    ctx.beginPath();
    ctx.moveTo(xx, pad.top);
    ctx.lineTo(xx, height - pad.bottom);
    ctx.stroke();
  });

  // Draw chart axes.
  ctx.strokeStyle = "#8aa5c8";
  ctx.beginPath();
  ctx.moveTo(pad.left, pad.top);
  ctx.lineTo(pad.left, height - pad.bottom);
  ctx.lineTo(width - pad.right, height - pad.bottom);
  ctx.stroke();

  // Reusable helper for line series.
  const drawLine = (data, color) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = 2.2;
    ctx.beginPath();
    data.forEach((value, i) => {
      if (i === 0) {
        ctx.moveTo(x(i), y(value));
      } else {
        ctx.lineTo(x(i), y(value));
      }
    });
    ctx.stroke();
  };

  // Draw the two series.
  drawLine(rows.map((row) => row.requiredNew), "#bf2a46");
  drawLine(rows.map((row) => row.actualNew), "#0f7a8e");

  // Draw point markers (downsampled for big datasets).
  const pointStep = rows.length <= 40 ? 1 : Math.ceil(rows.length / 40);
  const drawPoints = (data, color) => {
    ctx.fillStyle = color;
    data.forEach((value, i) => {
      if (i % pointStep !== 0 && i !== data.length - 1) {
        return;
      }
      ctx.beginPath();
      ctx.arc(x(i), y(value), 2.5, 0, Math.PI * 2);
      ctx.fill();
    });
  };

  drawPoints(rows.map((row) => row.requiredNew), "#bf2a46");
  drawPoints(rows.map((row) => row.actualNew), "#0f7a8e");

  // Manual legend.
  ctx.fillStyle = "#bf2a46";
  ctx.fillRect(pad.left, 10, 16, 3);
  ctx.fillStyle = "#0f7a8e";
  ctx.fillRect(pad.left + 220, 10, 16, 3);

  ctx.fillStyle = "#233f60";
  ctx.font = "13px 'Space Grotesk', sans-serif";
  ctx.fillText("Required new participants", pad.left + 22, 15);
  ctx.fillText("Actual new participants", pad.left + 242, 15);

  // Optional collapse marker line.
  const collapsedIndex = rows.findIndex((row) => row.status === "COLLAPSED");
  if (collapsedIndex >= 0) {
    ctx.strokeStyle = "#bf2a46";
    ctx.setLineDash([5, 4]);
    ctx.beginPath();
    ctx.moveTo(x(collapsedIndex), pad.top);
    ctx.lineTo(x(collapsedIndex), height - pad.bottom);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = "#bf2a46";
    ctx.font = "12px 'IBM Plex Mono', monospace";
    const collapseText = `collapse @ cycle ${rows[collapsedIndex].cycle}`;
    const collapseWidth = ctx.measureText(collapseText).width;
    const proposedX = x(collapsedIndex) + 6;
    const clampedX = Math.min(proposedX, width - pad.right - collapseWidth - 4);
    ctx.fillText(collapseText, Math.max(pad.left + 6, clampedX), pad.top + 14);
  }

  // X tick labels.
  ctx.fillStyle = "#4d6685";
  ctx.font = "12px 'IBM Plex Mono', monospace";
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  tickIndexes.forEach((idx) => {
    const xx = x(idx);
    const cycleLabel = rows[idx].cycle;
    ctx.fillText(String(cycleLabel), xx, height - pad.bottom + 8);
  });
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";

  // Axis labels.
  ctx.fillStyle = "#233f60";
  ctx.font = "13px 'Space Grotesk', sans-serif";
  ctx.save();
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.translate(24, pad.top + plotHeight / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.fillText("Participants per cycle", 0, 0);
  ctx.restore();
  const xAxisText = "Cycle number";
  const xAxisTextWidth = ctx.measureText(xAxisText).width;
  ctx.fillText(xAxisText, pad.left + (plotWidth - xAxisTextWidth) / 2, height - 8);

  // Tiny note in top-right so users know how many cycles were plotted.
  const last = rows[rows.length - 1];
  ctx.fillStyle = "#4d6685";
  ctx.font = "12px 'IBM Plex Mono', monospace";
  ctx.fillText(`Cycles shown: 1-${last ? last.cycle : 0}`, width - 190, 18);
}

// Builds the body of the per-cycle results table.
function renderTable(rows) {
  resultsBody.innerHTML = rows
    .map((row) => {
      const statusClass = row.status === "COLLAPSED" ? "status-collapsed" : "status-ok";
      const rowClass = row.status === "COLLAPSED" ? "collapsed-row" : "";

      return `
        <tr class="${rowClass}">
          <td>${row.cycle}</td>
          <td>${formatParticipants(row.activeParticipants)}</td>
          <td>${formatParticipants(row.requiredNew)}</td>
          <td>${formatParticipants(row.actualNew)}</td>
          <td class="${statusClass}">${row.status}</td>
        </tr>
      `;
    })
    .join("");
}

// Fixed educational summary text.
function renderSummary() {
  summary.textContent = summaryText;
}

// Runs the full flow: read settings, simulate, then render outputs.
function runSimulation() {
  const params = readInputs();
  const result = simulate(params);
  renderSummary();
  renderTable(result.rows);
  drawChart(result.rows);
}

// As sliders move, we update the small value labels immediately.
sliderIds.forEach((id) => {
  sliders[id].addEventListener("input", setOutputText);
});

// Run button uses current slider values.
runBtn.addEventListener("click", runSimulation);

// Reset restores defaults and then runs once so output matches defaults.
resetBtn.addEventListener("click", () => {
  resetInputs();
  runSimulation();
});

// Redraw chart on window resize so canvas stays crisp.
window.addEventListener("resize", () => {
  const params = readInputs();
  const result = simulate(params);
  drawChart(result.rows);
});

// First page load setup.
resetInputs();
runSimulation();
