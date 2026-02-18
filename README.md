# ponzi-collapse-demo
Interactive educational JavaScript demo that visualizes why schemes paying old participants only from new participants are mathematically unsustainable and eventually collapse.

## Purpose

This simulator is anti-scam educational material.

- Everyone contributes `1` abstract unit.
- Payout pressure grows each cycle.
- The only new units come from new participants.
- As recruitment slows and people drop out, the system runs out of newcomers.

## Run Locally

From the project folder:

```bash
python3 -m http.server 8080
```

Then open [http://localhost:8080](http://localhost:8080).

## Controls

- `Promised return per cycle (%)`: how fast obligations grow each cycle.
- `Dropout per cycle (%)`: percent of active participants leaving each cycle.
- `Recruitment slows over time (%)`: decay applied to recruitment potential each cycle.
- `Initial participants`: starting size of the system.
- `Cycles to simulate`: number of model steps to run.
- `Population cap`: hard upper bound on total possible participants.

Note: a cycle is an abstract model step (not a fixed real-world time period).

## Model (Simple)

Per cycle:

1. Obligations generate required newcomers: `requiredNew = obligations * promisedReturn`.
2. Active participants shrink by dropout.
3. Recruitment potential decays over time.
4. Actual newcomers are limited by recruitment potential and remaining population.

Collapse is triggered when either:

- `requiredNew > actualNew`, or
- cumulative required participants exceed `populationCap`.

## Files

- `index.html` - UI layout
- `styles.css` - styling
- `app.js` - simulation, table rendering, chart rendering
