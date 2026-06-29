document.addEventListener('DOMContentLoaded', () => {
  // DOM Elements - Controls
  const inputSpend = document.getElementById('input-spend');
  const inputServers = document.getElementById('input-servers');
  const inputUnitCost = document.getElementById('input-unit-cost');
  const inputDbCost = document.getElementById('input-db-cost');

  // DOM Elements - Labels
  const valSpend = document.getElementById('val-spend');
  const valServers = document.getElementById('val-servers');
  const valUnitCost = document.getElementById('val-unit-cost');
  const valDbCost = document.getElementById('val-db-cost');

  // DOM Elements - Results (Traditional)
  const tradInfra = document.getElementById('trad-infra');
  const tradDb = document.getElementById('trad-db');
  const tradWastage = document.getElementById('trad-wastage');
  const tradTotal = document.getElementById('trad-total');

  // DOM Elements - Results (oxideLLM)
  const oxInfra = document.getElementById('ox-infra');
  const oxDb = document.getElementById('ox-db');
  const oxWastage = document.getElementById('ox-wastage');
  const oxTotal = document.getElementById('ox-total');

  // DOM Elements - Savings Banner
  const savingsPercentText = document.getElementById('savings-percent');
  const savingsMonthlyText = document.getElementById('savings-monthly');
  const savingsYearlyText = document.getElementById('savings-yearly');

  // Helper function to format currency
  const formatCurrency = (value) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      maximumFractionDigits: 0
    }).format(value);
  };

  // Calculation Engine
  const calculateTCO = () => {
    // 1. Gather raw inputs
    const spend = parseInt(inputSpend.value, 10);
    const servers = parseInt(inputServers.value, 10);
    const unitCost = parseInt(inputUnitCost.value, 10);
    const dbCost = parseInt(inputDbCost.value, 10);

    // 2. Update dynamic labels
    valSpend.textContent = formatCurrency(spend);
    valServers.textContent = servers;
    valUnitCost.textContent = formatCurrency(unitCost);
    valDbCost.textContent = formatCurrency(dbCost);

    // 3. Traditional Costs
    const traditionalInfraCost = servers * unitCost;
    const traditionalDbCost = dbCost;
    // 8% token wastage due to uncancelled streams when clients disconnect
    const traditionalWastageCost = Math.round(spend * 0.08); 
    const traditionalTotalTco = traditionalInfraCost + traditionalDbCost + traditionalWastageCost;

    // 4. oxideLLM Costs
    // 4x consolidation efficiency: requires 4x fewer servers (minimum of 1)
    const oxideServers = Math.ceil(servers / 4);
    const oxideInfraCost = oxideServers * unitCost;
    const oxideDbCost = 0; // Invariant: No database on hot path
    // 0.5% wastage due to aggressive immediate upstream stream-cancel triggers
    const oxideWastageCost = Math.round(spend * 0.005); 
    const oxideTotalTco = oxideInfraCost + oxideDbCost + oxideWastageCost;

    // 5. Savings calculations
    const monthlySavings = traditionalTotalTco - oxideTotalTco;
    const yearlySavings = monthlySavings * 12;
    const savingsPercent = traditionalTotalTco > 0 
      ? Math.round((monthlySavings / traditionalTotalTco) * 1000) / 10 
      : 0;

    // 6. Update results in the DOM
    tradInfra.textContent = formatCurrency(traditionalInfraCost);
    tradDb.textContent = formatCurrency(traditionalDbCost);
    tradWastage.textContent = formatCurrency(traditionalWastageCost);
    tradTotal.textContent = formatCurrency(traditionalTotalTco);

    oxInfra.textContent = formatCurrency(oxideInfraCost);
    oxDb.textContent = formatCurrency(oxideDbCost);
    oxWastage.textContent = formatCurrency(oxideWastageCost);
    oxTotal.textContent = formatCurrency(oxideTotalTco);

    // 7. Update Savings Banner with a micro-fade effect
    savingsPercentText.textContent = `${savingsPercent}%`;
    savingsMonthlyText.textContent = `Modeled savings: ${formatCurrency(monthlySavings)} / month`;
    savingsYearlyText.textContent = `(${formatCurrency(yearlySavings)} modeled per year)`;
  };

  // Add event listeners for slider input tracking
  [inputSpend, inputServers, inputUnitCost, inputDbCost].forEach(input => {
    input.addEventListener('input', calculateTCO);
  });

  // Run initial calculation on load
  calculateTCO();
});
