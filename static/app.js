/**
 * Simplex Consensus Trace Visualizer
 * Professional visualization tool for Commonware Simplex consensus protocol
 * 
 * Timeline visualization of consensus events across nodes
 */

(() => {
  'use strict';

  // ============================================================================
  // State
  // ============================================================================

  const state = {
    events: [],
    edges: [],
    views: [],
    segments: [],
    meta: null,
    
    // Viewport
    viewStart: 0n,
    viewEnd: 0n,
    centerNs: 0n,
    spanNs: 0n,
    
    // Selection
    activeSegment: 0,
    selectedEvent: null,
    hoveredEvent: null,
    
    // Filters
    enabledKinds: new Set(),
    enabledCategories: new Set(['consensus', 'broadcast', 'resolver', 'marshal', 'batcher']),
    enabledNodes: new Set(),
    
    // UI
    isDragging: false,
    lastVisible: [],
    
    // View mode: 'standard' or 'pipelining'
    viewMode: 'standard',
  };

  // ============================================================================
  // DOM Elements
  // ============================================================================

  const elements = {
    timeline: document.getElementById('timeline'),
    minimap: document.getElementById('minimap'),
    minimapViewport: document.getElementById('minimap-viewport'),
    pipeline: document.getElementById('pipeline'),
    segmentNav: document.getElementById('segment-nav'),
    viewModeToggle: document.getElementById('view-mode-toggle'),
    categoryFilters: document.getElementById('category-filters'),
    kindFilters: document.getElementById('kind-filters'),
    nodeFilters: document.getElementById('node-filters'),
    tooltip: document.getElementById('tooltip'),
    status: document.getElementById('status'),
    viewInfo: document.getElementById('view-info'),
    timeInfo: document.getElementById('time-info'),
  };

  const ctx = elements.timeline.getContext('2d');
  const minimapCtx = elements.minimap.getContext('2d');
  const pipelineCtx = elements.pipeline.getContext('2d');

  // ============================================================================
  // Color Definitions
  // ============================================================================

  const categoryColors = {
    consensus: '#58a6ff',
    network: '#a371f7',
    storage: '#6e7681',
    resolver: '#39c5cf',
    marshal: '#3fb950',
    batcher: '#f0883e',
    broadcast: '#38bdf8',
  };

  const defaultKindColors = {
    1: '#60a5fa',   // LeaderElected
    2: '#34d399',   // ProposalRequested
    3: '#2dd4bf',   // ProposalReceived
    4: '#22d3ee',   // ProposalVerify
    5: '#a78bfa',   // NotarizeBroadcast
    6: '#c4b5fd',   // NotarizedBuilt
    7: '#ddd6fe',   // NotarizationReceived
    8: '#fb923c',   // NullifyBroadcast
    9: '#fdba74',   // NullifiedBuilt
    10: '#fed7aa',  // NullificationReceived
    11: '#f87171',  // FinalizeBroadcast
    12: '#fca5a5',  // FinalizedBuilt
    13: '#fecaca',  // FinalizationReceived
    14: '#fbbf24',  // CertificationAttempt
    21: '#4ade80',  // MarshalCached
    22: '#86efac',  // MarshalSubscriber
    23: '#bbf7d0',  // MarshalBroadcast
    31: '#94a3b8',  // BatcherNoVerifier
    32: '#cbd5e1',  // BatcherSkipDuplicate
    41: '#38bdf8',  // BufferedBroadcast
    42: '#7dd3fc',  // BufferedGet
    43: '#bae6fd',  // BufferedSubscribe
    44: '#e0f2fe',  // BufferedNetwork
    51: '#e879f9',  // ResolverCancel
    52: '#f0abfc',  // ResolverRetain
    255: '#9ca3af', // Other
  };

  // Important consensus events (shown by default)
  // ProposalRequested=2, NotarizeBroadcast=5, NotarizedBuilt=6,
  // NullifyBroadcast=8, NullifiedBuilt=9, FinalizeBroadcast=11, FinalizedBuilt=12
  const defaultEnabledKinds = new Set([2, 5, 6, 8, 9, 11, 12]);

  // ============================================================================
  // Data Fetching
  // ============================================================================

  async function fetchData() {
    try {
      const [eventsRes, edgesRes, viewsRes, segmentsRes, metaRes] = await Promise.all([
        fetch('/api/events?limit=50000'),
        fetch('/api/edges'),
        fetch('/api/views'),
        fetch('/api/segments'),
        fetch('/api/meta'),
      ]);

      state.events = await eventsRes.json();
      state.edges = await edgesRes.json();
      state.views = await viewsRes.json();
      state.segments = await segmentsRes.json();
      state.meta = await metaRes.json();

      // Initialize enabled kinds from metadata
      if (state.meta.event_kinds) {
        state.meta.event_kinds.forEach(k => {
          if (defaultEnabledKinds.has(k.tag)) {
            state.enabledKinds.add(k.tag);
          }
        });
      }

      // Initialize enabled nodes
      if (state.meta.nodes) {
        state.meta.nodes.forEach(n => state.enabledNodes.add(n));
      }

      computeViewport();
      buildUI();
      draw();

      elements.status.textContent = `${state.events.length.toLocaleString()} events`;
    } catch (err) {
      console.error('Failed to fetch data:', err);
      elements.status.textContent = 'Error loading data';
    }
  }

  // ============================================================================
  // Viewport Management
  // ============================================================================

  function updateViewportFromCenter() {
    state.viewStart = state.centerNs - state.spanNs / 2n;
    state.viewEnd = state.centerNs + state.spanNs / 2n;
  }

  function computeViewport() {
    if (state.events.length === 0) return;
    const minTs = BigInt(state.meta.min_ts_ns), maxTs = BigInt(state.meta.max_ts_ns);
    state.spanNs = maxTs - minTs || 1n;
    const pad = state.spanNs / 20n;
    state.viewStart = minTs - pad;
    state.viewEnd = maxTs + pad;
    state.spanNs = state.viewEnd - state.viewStart;
    state.centerNs = state.viewStart + state.spanNs / 2n;
  }

  function setViewportFromSegment(segmentIndex) {
    const segment = state.segments[segmentIndex];
    if (!segment) return;
    state.activeSegment = segmentIndex;
    const start = BigInt(segment.start_ns), end = BigInt(segment.end_ns);
    const pad = (end - start) / 20n;
    state.viewStart = start - pad;
    state.viewEnd = end + pad;
    state.spanNs = state.viewEnd - state.viewStart;
    state.centerNs = state.viewStart + state.spanNs / 2n;
    updateSegmentNav();
    draw();
  }

  function pan(fraction) {
    state.centerNs += (state.spanNs * BigInt(Math.floor(fraction * 1000))) / 1000n;
    updateViewportFromCenter();
    draw();
  }

  function zoom(factor) {
    const newSpan = (state.spanNs * BigInt(Math.floor(factor * 1000))) / 1000n;
    const minSpan = 1_000_000n, maxSpan = BigInt(state.meta.max_ts_ns) - BigInt(state.meta.min_ts_ns) + 1n;
    state.spanNs = newSpan < minSpan ? minSpan : newSpan > maxSpan ? maxSpan : newSpan;
    updateViewportFromCenter();
    draw();
  }

  function timeToX(ts, width) {
    if (ts < state.viewStart || ts > state.viewEnd) return null;
    const range = state.viewEnd - state.viewStart;
    return range === 0n ? 0 : Number((ts - state.viewStart) * BigInt(width) / range);
  }

  function xToTime(x, width) {
    return state.viewStart + ((state.viewEnd - state.viewStart) * BigInt(Math.floor(x))) / BigInt(width);
  }

  // ============================================================================
  // UI Building
  // ============================================================================

  function buildUI() {
    buildSegmentNav();
    buildCategoryFilters();
    buildKindFilters();
    buildNodeFilters();
  }

  function buildSegmentNav() {
    elements.segmentNav.innerHTML = state.segments.map((seg, idx) => 
      `<button class="segment-btn ${idx === state.activeSegment ? 'active' : ''}" 
        title="Segment ${idx + 1}: ${seg.event_count} events, views ${seg.views[0] || '?'}-${seg.views[seg.views.length - 1] || '?'}">
        ${idx + 1}<span class="event-count">${formatNumber(seg.event_count)}</span></button>`
    ).join('');
    elements.segmentNav.querySelectorAll('.segment-btn').forEach((btn, idx) => 
      btn.addEventListener('click', () => setViewportFromSegment(idx)));
  }

  function updateSegmentNav() {
    elements.segmentNav.querySelectorAll('.segment-btn').forEach((btn, idx) => 
      btn.classList.toggle('active', idx === state.activeSegment));
  }

  function toggleSet(set, key, checked) {
    checked ? set.add(key) : set.delete(key);
    draw();
  }

  function buildCategoryFilters() {
    elements.categoryFilters.innerHTML = '';
    ['consensus', 'broadcast', 'resolver', 'marshal', 'batcher', 'network', 'storage'].forEach(cat => {
      const count = state.events.filter(e => e.category.toLowerCase() === cat).length;
      elements.categoryFilters.appendChild(
        createFilterItem(cat, categoryColors[cat], count, state.enabledCategories.has(cat), 
          checked => toggleSet(state.enabledCategories, cat, checked))
      );
    });
  }

  function buildKindFilters() {
    elements.kindFilters.innerHTML = '';
    if (!state.meta?.event_kinds) return;
    
    state.meta.event_kinds.forEach(k => {
      const count = state.events.filter(e => e.kind_tag === k.tag).length;
      if (count === 0) return;
      elements.kindFilters.appendChild(
        createFilterItem(k.label, k.color || defaultKindColors[k.tag], count, state.enabledKinds.has(k.tag),
          checked => toggleSet(state.enabledKinds, k.tag, checked))
      );
    });
  }

  function buildNodeFilters() {
    elements.nodeFilters.innerHTML = '';
    if (!state.meta?.nodes) return;
    state.meta.nodes.forEach(nodeId => {
      const count = state.events.filter(e => e.raw.node_id === nodeId).length;
      elements.nodeFilters.appendChild(
        createFilterItem(`Node ${nodeId}`, getNodeColor(nodeId), count, state.enabledNodes.has(nodeId),
          checked => toggleSet(state.enabledNodes, nodeId, checked))
      );
    });
  }

  function createFilterItem(label, color, count, checked, onChange) {
    const item = document.createElement('label');
    item.className = 'filter-item';
    item.innerHTML = `<input type="checkbox" ${checked ? 'checked' : ''}>
      <span class="color-dot" style="background:${color}"></span>
      <span class="label">${label}</span>
      <span class="count">${formatNumber(count)}</span>`;
    item.querySelector('input').addEventListener('change', e => onChange(e.target.checked));
    return item;
  }

  // ============================================================================
  // Drawing
  // ============================================================================

  function draw() {
    resize();
    drawTimeline();
    drawMinimap();
    drawPipeline();
    updateFooter();
  }

  function resize() {
    const dpr = window.devicePixelRatio || 1;
    
    // Timeline
    elements.timeline.width = elements.timeline.clientWidth * dpr;
    elements.timeline.height = elements.timeline.clientHeight * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    
    // Minimap
    elements.minimap.width = elements.minimap.clientWidth * dpr;
    elements.minimap.height = elements.minimap.clientHeight * dpr;
    minimapCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    
    // Pipeline
    elements.pipeline.width = elements.pipeline.clientWidth * dpr;
    elements.pipeline.height = elements.pipeline.clientHeight * dpr;
    pipelineCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  // ============================================================================
  // Consensus View Drawing - Dispatcher
  // ============================================================================
  const VIEW_SEPARATOR_STYLE = Object.freeze({
    dash: [6, 4],
    lineWidth: 1.5,
    stroke: '#8b949e',
    labelFont: '10px "SF Mono", monospace',
    labelColor: '#c9d1d9',
  });
  function drawCenteredText(text, w, h) {
    ctx.fillStyle = '#6e7681';
    ctx.font = '14px system-ui';
    ctx.textAlign = 'center';
    ctx.fillText(text, w / 2, h / 2);
  }

  function drawTimeline() {
    if (state.viewMode === 'pipelining') {
      drawTimelinePipelining();
    } else {
      drawTimelineStandard();
    }
  }

  // ============================================================================
  // Standard Timeline View (Original)
  // ============================================================================

  function drawTimelineStandard() {
    const w = elements.timeline.clientWidth, h = elements.timeline.clientHeight;
    ctx.clearRect(0, 0, w, h);
    
    if (state.events.length === 0) return drawCenteredText('No events loaded', w, h);

    const nodes = Array.from(state.enabledNodes).sort((a, b) => a - b);
    const availableHeight = h - 60;
    const laneHeight = Math.max(20, Math.min(80, nodes.length > 0 ? availableHeight / nodes.length : 60));
    const topPadding = 30;

    // Draw view bands
    drawViewBands(w, h, topPadding);
    
    // Draw grid lines and node labels
    drawGrid(nodes, laneHeight, w, h, topPadding);
    
    const visibleEvents = getVisibleEvents();

    // Draw edges/arrows
    drawEdges(visibleEvents, nodes, laneHeight, w, topPadding);
    
    // Draw events
    state.lastVisible = [];
    drawEvents(visibleEvents, nodes, laneHeight, w, topPadding);
    
    // Draw time axis
    drawTimeAxis(w, h);
  }

  // ============================================================================
  // Pipelining View - Stacked Panel Design
  // Top panel: Finalization events (completing previous views)
  // Bottom panel: Proposal/Notarization events (current view work)
  // ============================================================================

  const FINALIZE_KIND_TAGS = new Set([11, 12]); // FinalizeBroadcast=11, FinalizedBuilt=12

  function getVisibleEvents() {
    return state.events.filter(ev => {
      const ts = BigInt(ev.raw.timestamp_ns);
      return ts >= state.viewStart && ts <= state.viewEnd && 
             state.enabledKinds.has(ev.kind_tag) &&
             state.enabledCategories.has(ev.category.toLowerCase()) &&
             state.enabledNodes.has(ev.raw.node_id);
    });
  }

  function drawTimelinePipelining() {
    const w = elements.timeline.clientWidth, h = elements.timeline.clientHeight;
    ctx.clearRect(0, 0, w, h);
    
    if (w === 0 || h === 0) return;
    if (state.events.length === 0) return drawCenteredText('No events loaded', w, h);

    const nodes = Array.from(state.enabledNodes).sort((a, b) => a - b);
    if (nodes.length === 0) return drawCenteredText('No nodes enabled', w, h);

    // Layout: two stacked panels - scalable for many validators
    const topPanelRatio = 0.30, dividerHeight = 2, headerHeight = 18, timeAxisHeight = 25, labelWidth = 65;
    
    const topPanelHeight = Math.floor((h - timeAxisHeight) * topPanelRatio);
    const bottomPanelHeight = h - topPanelHeight - dividerHeight - timeAxisHeight;
    const topPanelY = 0, bottomPanelY = topPanelHeight + dividerHeight;
    
    // Scalable lane heights with min/max bounds
    const topLaneHeight = Math.max(14, Math.min(30, (topPanelHeight - headerHeight) / nodes.length));
    const bottomLaneHeight = Math.max(14, Math.min(40, (bottomPanelHeight - headerHeight) / nodes.length));

    const allVisibleEvents = getVisibleEvents();
    const topPanelEvents = allVisibleEvents.filter(ev => FINALIZE_KIND_TAGS.has(ev.kind_tag));
    const bottomPanelEvents = allVisibleEvents.filter(ev => !FINALIZE_KIND_TAGS.has(ev.kind_tag));

    state.lastVisible = [];

    // Top Panel (Finalization)
    drawPipelinePanel({
      events: topPanelEvents,
      nodes,
      panelY: topPanelY,
      panelHeight: topPanelHeight,
      laneHeight: topLaneHeight,
      headerHeight,
      labelWidth,
      canvasWidth: w,
      label: 'FINALIZE',
      labelColor: '#f87171',
      bgColor: 'rgba(248, 113, 113, 0.03)',
      gridColor: 'rgba(248, 113, 113, 0.15)',
      broadcastTags: new Set([11]),
    });

    // Divider
    ctx.fillStyle = '#2a3441';
    ctx.fillRect(0, topPanelHeight, w, dividerHeight);

    // Bottom Panel (Proposal/Notarization)
    drawPipelinePanel({
      events: bottomPanelEvents,
      nodes,
      panelY: bottomPanelY,
      panelHeight: bottomPanelHeight,
      laneHeight: bottomLaneHeight,
      headerHeight,
      labelWidth,
      canvasWidth: w,
      label: 'PROPOSE / NOTARIZE',
      labelColor: '#58a6ff',
      bgColor: 'rgba(88, 166, 255, 0.02)',
      gridColor: 'rgba(88, 166, 255, 0.1)',
      broadcastTags: new Set([2, 5, 8]),
    });

    // Shared time axis
    drawPipelineTimeAxis(w, h, timeAxisHeight, labelWidth);

    // View alignment markers
    drawPipelineViewMarkers(allVisibleEvents, w, topPanelHeight, bottomPanelY, bottomPanelHeight, labelWidth);
  }

  function drawPipelinePanel({ events, nodes, panelY, panelHeight, laneHeight, headerHeight, labelWidth, canvasWidth, label, labelColor, bgColor, gridColor, broadcastTags }) {
    const contentY = panelY + headerHeight;
    
    // Panel background
    ctx.fillStyle = bgColor;
    ctx.fillRect(labelWidth, panelY, canvasWidth - labelWidth, panelHeight);
    
    // Panel header
    ctx.save();
    ctx.fillStyle = labelColor;
    ctx.font = 'bold 10px "SF Mono", monospace';
    ctx.textAlign = 'left';
    ctx.fillText(label, 8, panelY + 13);
    ctx.restore();
    
    // Node labels and grid - scale font with lane height
    const nodeFontSize = Math.max(8, Math.min(11, laneHeight / 2));
    ctx.save();
    nodes.forEach((nodeId, idx) => {
      const y = contentY + idx * laneHeight + laneHeight / 2;
      
      ctx.fillStyle = '#6e7681';
      ctx.font = `${nodeFontSize}px "SF Mono", monospace`;
      ctx.textAlign = 'right';
      ctx.fillText(`N${nodeId}`, labelWidth - 8, y + nodeFontSize / 3);
      
      ctx.strokeStyle = gridColor;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(labelWidth, y);
      ctx.lineTo(canvasWidth, y);
      ctx.stroke();
    });
    ctx.restore();

    // Broadcast arrows - scale with lane height
    const radius = Math.max(2, Math.min(4, laneHeight / 5));
    const arrowLength = Math.min(Math.max(10, laneHeight / 2), 20, canvasWidth * 0.015);
    const arrowHeadSize = Math.max(2, Math.min(4, laneHeight / 6));
    
    events.forEach(ev => {
      if (!broadcastTags.has(ev.kind_tag)) return;
      
      const x = timeToX(BigInt(ev.raw.timestamp_ns), canvasWidth);
      if (x === null || x < labelWidth) return;
      
      const fromIdx = nodes.indexOf(ev.raw.node_id);
      if (fromIdx === -1) return;
      
      const fromY = contentY + fromIdx * laneHeight + laneHeight / 2;
      const color = getKindColor(ev.kind_tag);
      
      ctx.save();
      ctx.globalAlpha = 0.4;
      
      nodes.forEach((nodeId, toIdx) => {
        if (nodeId === ev.raw.node_id) return;
        const toY = contentY + toIdx * laneHeight + laneHeight / 2;
        drawArrow(x, fromY, x + arrowLength, toY, color, 0.4, arrowHeadSize);
      });
      
      ctx.restore();
    });

    // Event dots
    events.forEach(ev => {
      const x = timeToX(BigInt(ev.raw.timestamp_ns), canvasWidth);
      if (x === null || x < labelWidth) return;
      
      const nodeIdx = nodes.indexOf(ev.raw.node_id);
      if (nodeIdx === -1) return;
      
      const y = contentY + nodeIdx * laneHeight + laneHeight / 2;
      const color = getKindColor(ev.kind_tag);
      
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
      
      if (ev === state.selectedEvent || ev === state.hoveredEvent) {
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.stroke();
      }
      
      state.lastVisible.push({ x, y, r: radius, event: ev });
    });
  }

  function drawPipelineTimeAxis(w, h, axisHeight, labelWidth) {
    const axisY = h - axisHeight;
    
    ctx.fillStyle = '#0f1419';
    ctx.fillRect(0, axisY, w, axisHeight);
    
    ctx.strokeStyle = '#2a3441';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(labelWidth, axisY + 2);
    ctx.lineTo(w, axisY + 2);
    ctx.stroke();
    
    const range = state.viewEnd - state.viewStart;
    const numTicks = Math.floor((w - labelWidth) / 100);
    
    ctx.fillStyle = '#6e7681';
    ctx.font = '10px "SF Mono", monospace';
    ctx.textAlign = 'center';
    
    for (let i = 0; i <= numTicks; i++) {
      const x = labelWidth + ((w - labelWidth) * i) / numTicks;
      const ts = state.viewStart + (range * BigInt(i)) / BigInt(numTicks);
      
      ctx.beginPath();
      ctx.moveTo(x, axisY + 2);
      ctx.lineTo(x, axisY + 6);
      ctx.stroke();
      
      const ms = Number(ts - BigInt(state.meta.min_ts_ns)) / 1_000_000;
      ctx.fillText(formatTime(ms), x, axisY + 18);
    }
  }

  function drawPipelineViewMarkers(events, w, topPanelHeight, bottomPanelY, bottomPanelHeight, labelWidth) {
    const viewStarts = new Map();
    
    events.forEach(ev => {
      if (ev.view === null || ev.view === undefined) return;
      const ts = BigInt(ev.raw.timestamp_ns);
      if (!viewStarts.has(ev.view) || ts < viewStarts.get(ev.view)) {
        viewStarts.set(ev.view, ts);
      }
    });
    
    ctx.save();
    ctx.globalAlpha = 1;
    ctx.setLineDash(VIEW_SEPARATOR_STYLE.dash);
    ctx.lineWidth = VIEW_SEPARATOR_STYLE.lineWidth;
    ctx.strokeStyle = VIEW_SEPARATOR_STYLE.stroke;
    ctx.font = VIEW_SEPARATOR_STYLE.labelFont;
    ctx.textAlign = 'left';
    
    viewStarts.forEach((ts, viewNum) => {
      const x = timeToX(ts, w);
      if (x === null || x < labelWidth) return;

      // Keep separator out of the shared time axis area at the bottom.
      const axisTop = bottomPanelY + bottomPanelHeight;
      const separatorEndY = Math.max(0, axisTop ); // leave 6px breathing room above labels
      
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, separatorEndY);
      ctx.stroke();
      
      ctx.fillStyle = VIEW_SEPARATOR_STYLE.labelColor;
      ctx.fillText(`v${viewNum}`, x + 4, topPanelHeight - 4);
    });
    
    ctx.setLineDash([]);
    ctx.restore();
  }

  function drawViewBands(w, h, topPadding) {
    // Find leader for each view: the node that has ProposalRequested (kind_tag=2) event
    const viewLeaders = new Map(); // view -> leader node_id
    
    state.events.forEach(ev => {
      if (ev.view === null || ev.view === undefined) return;
      if (ev.kind_tag === 2 && !viewLeaders.has(ev.view)) {
        viewLeaders.set(ev.view, ev.raw.node_id);
      }
    });

    // Build index of NotarizedBuilt (kind_tag=6) and NullifiedBuilt (kind_tag=9) events by node:view
    const nodeViewNotarizedBuilt = new Map(); // "node:view" -> timestamp_ns
    const nodeViewNullifiedBuilt = new Map(); // "node:view" -> timestamp_ns
    
    state.events.forEach(ev => {
      if (ev.view === null || ev.view === undefined) return;
      if (ev.kind_tag === 6) {
        const key = `${ev.raw.node_id}:${ev.view}`;
        if (!nodeViewNotarizedBuilt.has(key)) {
          nodeViewNotarizedBuilt.set(key, BigInt(ev.raw.timestamp_ns));
        }
      }
      if (ev.kind_tag === 9) {
        const key = `${ev.raw.node_id}:${ev.view}`;
        if (!nodeViewNullifiedBuilt.has(key)) {
          nodeViewNullifiedBuilt.set(key, BigInt(ev.raw.timestamp_ns));
        }
      }
    });

    // Build view start times: View V start = leader's NotarizedBuilt(V-1) or NullifiedBuilt(V-1)
    const viewStartTimes = new Map(); // view -> start timestamp
    
    viewLeaders.forEach((leader, viewNum) => {
      const prevView = viewNum - 1;
      const key = `${leader}:${prevView}`;
      const notarizedBuilt = nodeViewNotarizedBuilt.get(key);
      const nullifiedBuilt = nodeViewNullifiedBuilt.get(key);
      // Use NotarizedBuilt if available, otherwise NullifiedBuilt (after nullification)
      const viewStart = notarizedBuilt !== undefined ? notarizedBuilt : nullifiedBuilt;
      if (viewStart !== undefined) {
        viewStartTimes.set(viewNum, viewStart);
      }
    });

    // Build view groups with proper start times
    const viewGroups = new Map();
    
    state.views.forEach(v => {
      const viewStart = viewStartTimes.get(v.view);
      if (viewStart !== undefined) {
        const finalized = v.finalized_ns ? BigInt(v.finalized_ns) : null;
        const notarized = v.notarized_ns ? BigInt(v.notarized_ns) : null;
        
        if (!viewGroups.has(v.view)) {
          viewGroups.set(v.view, { 
            start: viewStart, 
            end: finalized || notarized || viewStart, 
            view: v.view 
          });
        } else {
          const existing = viewGroups.get(v.view);
          // Use latest finalization/notarization across all nodes
          const thisEnd = finalized || notarized || viewStart;
          if (thisEnd > existing.end) {
            existing.end = thisEnd;
          }
        }
      }
    });

    const bands = Array.from(viewGroups.values()).sort((a, b) => 
      a.start < b.start ? -1 : a.start > b.start ? 1 : 0
    );

    bands.forEach((band, idx) => {
      const x1 = timeToX(band.start, w);
      
      if (x1 === null) return;
      
      ctx.save();
      ctx.setLineDash(VIEW_SEPARATOR_STYLE.dash);
      ctx.lineWidth = VIEW_SEPARATOR_STYLE.lineWidth;
      ctx.strokeStyle = VIEW_SEPARATOR_STYLE.stroke;
      ctx.font = VIEW_SEPARATOR_STYLE.labelFont;
      ctx.fillStyle = VIEW_SEPARATOR_STYLE.labelColor;
      ctx.textAlign = 'left';

      // View label
      ctx.fillText(`v${band.view}`, x1 + 4, topPadding + 12);
      
      // Dashed separator line at view start
      ctx.beginPath();
      ctx.moveTo(x1, topPadding);
      ctx.lineTo(x1, h - 24);
      ctx.stroke();

      ctx.setLineDash([]);
      ctx.restore();
    });
  }

  function drawGrid(nodes, laneHeight, w, h, topPadding) {
    ctx.strokeStyle = '#1e2632';
    ctx.lineWidth = 1;
    
    nodes.forEach((nodeId, idx) => {
      const y = topPadding + laneHeight * idx + laneHeight / 2;
      
      // Horizontal lane line
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
      
      // Node label - scale font with lane height
      const fontSize = Math.max(9, Math.min(12, laneHeight / 5));
      ctx.fillStyle = '#6e7681';
      ctx.font = `${fontSize}px "SF Mono", monospace`;
      ctx.fillText(`N${nodeId}`, 8, y + fontSize / 3);
    });
  }

  function drawEdges(events, nodes, laneHeight, w, topPadding) {
    // Draw arrows for broadcasts (votes + proposals)
    // ProposalRequested=2, NotarizeBroadcast=5, NullifyBroadcast=8, FinalizeBroadcast=11
    const broadcastTags = new Set([2, 5, 8, 11]);
    
    events.forEach(ev => {
      if (!broadcastTags.has(ev.kind_tag)) return;
      
      const x = timeToX(BigInt(ev.raw.timestamp_ns), w);
      if (x === null) return;
      
      const fromIdx = nodes.indexOf(ev.raw.node_id);
      if (fromIdx === -1) return;
      
      const yFrom = topPadding + laneHeight * fromIdx + laneHeight / 2;
      const color = getKindColor(ev.kind_tag);
      
      // Fan out arrows to other nodes - scale with lane height
      const arrowLength = Math.min(Math.max(15, laneHeight / 2), 30, w * 0.02);
      
      nodes.forEach((nodeId, toIdx) => {
        if (nodeId === ev.raw.node_id) return;
        
        const yTo = topPadding + laneHeight * toIdx + laneHeight / 2;
        const x2 = x + arrowLength;
        
        drawArrow(x, yFrom, x2, yTo, color, 0.4);
      });
    });
  }

  function drawArrow(x1, y1, x2, y2, color, alpha = 1, headSize = 4) {
    ctx.strokeStyle = color;
    ctx.globalAlpha = alpha;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
    
    const angle = Math.atan2(y2 - y1, x2 - x1);
    ctx.beginPath();
    ctx.moveTo(x2, y2);
    ctx.lineTo(x2 - headSize * Math.cos(angle - Math.PI / 6), y2 - headSize * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(x2 - headSize * Math.cos(angle + Math.PI / 6), y2 - headSize * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
    ctx.globalAlpha = 1;
  }

  function drawEvents(events, nodes, laneHeight, w, topPadding) {
    // Calculate dynamic radius based on zoom level and lane height
    const baseRadius = Math.max(2, Math.min(4, laneHeight / 20));
    const zoomFactor = Math.max(1, 4 / Math.sqrt(Number(state.spanNs) / 1e6 + 1));
    const radius = Math.min(baseRadius * zoomFactor, Math.min(8, laneHeight / 3));
    
    events.forEach(ev => {
      const x = timeToX(BigInt(ev.raw.timestamp_ns), w);
      if (x === null) return;
      
      const nodeIdx = nodes.indexOf(ev.raw.node_id);
      if (nodeIdx === -1) return;
      
      const y = topPadding + laneHeight * nodeIdx + laneHeight / 2;
      const color = getKindColor(ev.kind_tag);
      
      // Draw event dot
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
      
      // Highlight selected/hovered
      if (ev === state.selectedEvent || ev === state.hoveredEvent) {
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.stroke();
      }
      
      state.lastVisible.push({ x, y, r: radius, event: ev });
    });
  }

  function drawTimeAxis(w, h) {
    const axisY = h - 20;
    
    // Axis line
    ctx.strokeStyle = '#2a3441';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, axisY);
    ctx.lineTo(w, axisY);
    ctx.stroke();
    
    // Time markers
    const range = state.viewEnd - state.viewStart;
    const numTicks = Math.floor(w / 120);
    
    ctx.fillStyle = '#6e7681';
    ctx.font = '10px "SF Mono", monospace';
    ctx.textAlign = 'center';
    
    for (let i = 0; i <= numTicks; i++) {
      const x = (w * i) / numTicks;
      const ts = state.viewStart + (range * BigInt(i)) / BigInt(numTicks);
      
      // Tick mark
      ctx.beginPath();
      ctx.moveTo(x, axisY - 4);
      ctx.lineTo(x, axisY);
      ctx.stroke();
      
      // Time label
      const ms = Number(ts - BigInt(state.meta.min_ts_ns)) / 1_000_000;
      ctx.fillText(formatTime(ms), x, axisY + 12);
    }
  }

  function drawMinimap() {
    const w = elements.minimap.clientWidth;
    const h = elements.minimap.clientHeight;
    
    minimapCtx.clearRect(0, 0, w, h);
    
    if (state.events.length === 0) return;

    const minTs = BigInt(state.meta.min_ts_ns);
    const maxTs = BigInt(state.meta.max_ts_ns);
    const totalRange = maxTs - minTs;
    
    if (totalRange === 0n) return;

    // Draw event density
    const buckets = 200;
    const bucketCounts = new Array(buckets).fill(0);
    
    state.events.forEach(ev => {
      if (!state.enabledKinds.has(ev.kind_tag)) return;
      if (!state.enabledCategories.has(ev.category.toLowerCase())) return;
      
      const ts = BigInt(ev.raw.timestamp_ns);
      const bucket = Number((ts - minTs) * BigInt(buckets - 1) / totalRange);
      bucketCounts[Math.min(bucket, buckets - 1)]++;
    });
    
    const maxCount = Math.max(...bucketCounts, 1);
    const barWidth = w / buckets;
    
    bucketCounts.forEach((count, i) => {
      if (count === 0) return;
      
      const barHeight = (count / maxCount) * (h - 8);
      const x = i * barWidth;
      
      minimapCtx.fillStyle = 'rgba(88, 166, 255, 0.5)';
      minimapCtx.fillRect(x, h - 4 - barHeight, barWidth - 1, barHeight);
    });

    // Update viewport indicator position
    const vpStart = Number((state.viewStart - minTs) * BigInt(w) / totalRange);
    const vpEnd = Number((state.viewEnd - minTs) * BigInt(w) / totalRange);
    
    elements.minimapViewport.style.left = `${Math.max(0, vpStart)}px`;
    elements.minimapViewport.style.width = `${Math.min(w, vpEnd) - Math.max(0, vpStart)}px`;
  }

  // ============================================================================
  // Pipeline View - Visualizes Simplex Pipelining
  // ============================================================================

  function drawPipeline() {
    const w = elements.pipeline.clientWidth;
    const h = elements.pipeline.clientHeight;
    
    pipelineCtx.clearRect(0, 0, w, h);
    
    if (state.events.length === 0) return;

    // Find leader for each view: the node that has ProposalRequested (kind_tag=2) event
    const viewLeaders = new Map(); // view -> leader node_id
    
    state.events.forEach(ev => {
      if (ev.view === null || ev.view === undefined) return;
      // ProposalRequested (kind_tag=2) means this node is the leader for this view
      if (ev.kind_tag === 2 && !viewLeaders.has(ev.view)) {
        viewLeaders.set(ev.view, ev.raw.node_id);
      }
    });

    // Build index of events by node:view:kind_tag -> timestamp
    // We need each node's NotarizedBuilt(6) or NullifiedBuilt(9) per view for view start calculation
    const nodeViewEvents = new Map(); // "node:view:kind_tag" -> timestamp_ns
    
    state.events.forEach(ev => {
      if (ev.view === null || ev.view === undefined) return;
      
      const view = ev.view;
      const ts = BigInt(ev.raw.timestamp_ns);
      const nodeId = ev.raw.node_id;
      const kindTag = ev.kind_tag;
      
      // Index key events: NotarizeBroadcast(5), NotarizedBuilt(6), NullifiedBuilt(9), FinalizedBuilt(12)
      if (kindTag === 5 || kindTag === 6 || kindTag === 9 || kindTag === 12) {
        const key = `${nodeId}:${view}:${kindTag}`;
        // Use first occurrence for each event type
        if (!nodeViewEvents.has(key)) {
          nodeViewEvents.set(key, ts);
        }
      }
    });

    // Build phase timings per view using leader's events
    // For view V:
    // - Proposal: V leader's NotarizedBuilt(V-1) or NullifiedBuilt(V-1) -> V leader's NotarizeBroadcast(V)
    // - Notarization: V leader's NotarizeBroadcast(V) -> V+1 leader's NotarizedBuilt(V)
    // - Finalization: V+1 leader's NotarizedBuilt(V) -> V+1 leader's FinalizedBuilt(V)
    const viewPhases = new Map();
    
    viewLeaders.forEach((leader, viewNum) => {
      const phase = {
        view: viewNum,
        leader: leader,
        proposalStart: null,
        proposalEnd: null,
        notarizationStart: null,
        notarizationEnd: null,
        finalizationStart: null,
        finalizationEnd: null,
      };
      
      // View V start = V leader's NotarizedBuilt(6) or NullifiedBuilt(9) for view V-1
      const prevView = viewNum - 1;
      const notarizedBuiltPrev = nodeViewEvents.get(`${leader}:${prevView}:6`);  // NotarizedBuilt of V-1
      const nullifiedBuiltPrev = nodeViewEvents.get(`${leader}:${prevView}:9`);  // NullifiedBuilt of V-1
      // Use NotarizedBuilt if available, otherwise NullifiedBuilt (after nullification)
      const viewStart = notarizedBuiltPrev !== undefined ? notarizedBuiltPrev : nullifiedBuiltPrev;
      
      // Get V leader's NotarizeBroadcast for this view
      const notarizeBroadcast = nodeViewEvents.get(`${leader}:${viewNum}:5`);   // NotarizeBroadcast

      // Get V+1 leader's events for this view (V)
      const nextViewLeader = viewLeaders.get(viewNum + 1);
      let nextLeaderNotarizedBuilt = null;  // V+1 leader's NotarizedBuilt of view V
      let nextLeaderFinalizedBuilt = null;  // V+1 leader's FinalizedBuilt of view V
      
      if (nextViewLeader !== undefined) {
        nextLeaderNotarizedBuilt = nodeViewEvents.get(`${nextViewLeader}:${viewNum}:6`);   // NotarizedBuilt(V)
        nextLeaderFinalizedBuilt = nodeViewEvents.get(`${nextViewLeader}:${viewNum}:12`); // FinalizedBuilt(V)
      }
      
      // Proposal phase: V leader's NotarizedBuilt(V-1) or NullifiedBuilt(V-1) -> V leader's NotarizeBroadcast(V)
      if (viewStart !== undefined) {
        phase.proposalStart = viewStart;
        if (notarizeBroadcast !== undefined) {
          phase.proposalEnd = notarizeBroadcast;
        }
      }
      
      // Notarization phase: V leader's NotarizeBroadcast(V) -> V+1 leader's NotarizedBuilt(V)
      if (notarizeBroadcast !== undefined) {
        phase.notarizationStart = notarizeBroadcast;
        if (nextLeaderNotarizedBuilt !== undefined) {
          phase.notarizationEnd = nextLeaderNotarizedBuilt;
        }
      }
      
      // Finalization phase: V+1 leader's NotarizedBuilt(V) -> V+1 leader's FinalizedBuilt(V)
      if (nextLeaderNotarizedBuilt !== undefined) {
        phase.finalizationStart = nextLeaderNotarizedBuilt;
        if (nextLeaderFinalizedBuilt !== undefined) {
          phase.finalizationEnd = nextLeaderFinalizedBuilt;
        }
      }
      
      viewPhases.set(viewNum, phase);
    });

    // Convert to array and sort by view
    const phases = Array.from(viewPhases.values())
      .filter(p => p.proposalStart !== null)
      .sort((a, b) => Number(a.view) - Number(b.view));
    
    if (phases.length === 0) return;

    // Determine visible views based on current viewport
    const visiblePhases = phases.filter(p => {
      const start = p.proposalStart;
      const end = p.finalizationEnd || p.notarizationEnd || p.proposalEnd || p.proposalStart;
      return end >= state.viewStart && start <= state.viewEnd;
    });

    if (visiblePhases.length === 0) return;

    // Get sidebar width to offset pipeline to align with timeline
    const sidebar = document.getElementById('sidebar');
    const sidebarWidth = sidebar ? sidebar.offsetWidth : 0;

    // Drawing parameters - offset by sidebar width to match timeline below
    const topPadding = 4;
    const rowHeight = Math.min(16, (h - topPadding * 2) / Math.max(visiblePhases.length, 1));
    const barHeight = Math.max(rowHeight - 2, 6);

    // Drawable width (excluding sidebar area)
    const drawableWidth = w - sidebarWidth;

    // Use same coordinate system as timeline (maps to the drawable area after sidebar)
    const pipelineTimeToX = (ts) => {
      if (ts === null) return null;
      const range = state.viewEnd - state.viewStart;
      if (range === 0n) return sidebarWidth;
      const offset = ts - state.viewStart;
      return sidebarWidth + Number(offset * BigInt(drawableWidth) / range);
    };

    // Colors
    const colors = {
      proposal: '#34d399',       // green
      notarization: '#a78bfa',   // purple
      finalization: '#f87171',   // red
    };

    // Draw each view's phases
    visiblePhases.forEach((phase, idx) => {
      const y = topPadding + idx * rowHeight;
      
      // Proposal phase bar - only draw if we have both start and end
      if (phase.proposalStart !== null && phase.proposalEnd !== null) {
        const x1 = pipelineTimeToX(phase.proposalStart);
        const x2 = pipelineTimeToX(phase.proposalEnd);
        if (x1 !== null && x2 !== null) {
          const xStart = Math.max(sidebarWidth, x1);
          const xEnd = Math.min(w, x2);
          if (xEnd > xStart) {
            pipelineCtx.fillStyle = colors.proposal;
            pipelineCtx.globalAlpha = 0.8;
            pipelineCtx.fillRect(xStart, y, Math.max(xEnd - xStart, 2), barHeight);
          }
        }
      }
      
      // Notarization phase bar
      if (phase.notarizationStart !== null && phase.notarizationEnd !== null) {
        const x1 = pipelineTimeToX(phase.notarizationStart);
        const x2 = pipelineTimeToX(phase.notarizationEnd);
        if (x1 !== null && x2 !== null) {
          const xStart = Math.max(sidebarWidth, x1);
          const xEnd = Math.min(w, x2);
          if (xEnd > xStart) {
            pipelineCtx.fillStyle = colors.notarization;
            pipelineCtx.globalAlpha = 0.8;
            pipelineCtx.fillRect(xStart, y, Math.max(xEnd - xStart, 2), barHeight);
          }
        }
      }
      
      // Finalization phase bar
      if (phase.finalizationStart !== null && phase.finalizationEnd !== null) {
        const x1 = pipelineTimeToX(phase.finalizationStart);
        const x2 = pipelineTimeToX(phase.finalizationEnd);
        if (x1 !== null && x2 !== null) {
          const xStart = Math.max(sidebarWidth, x1);
          const xEnd = Math.min(w, x2);
          if (xEnd > xStart) {
            pipelineCtx.fillStyle = colors.finalization;
            pipelineCtx.globalAlpha = 0.8;
            pipelineCtx.fillRect(xStart, y, Math.max(xEnd - xStart, 2), barHeight);
          }
        }
      }
      
      pipelineCtx.globalAlpha = 1;
    });

    // Draw overlap indicators - show where N+1 proposal overlaps with N finalization
    pipelineCtx.save();
    pipelineCtx.globalAlpha = 0.3;
    pipelineCtx.strokeStyle = '#8b949e';
    pipelineCtx.setLineDash([6, 4]);
    pipelineCtx.lineWidth = 1.5;
    
    for (let i = 0; i < visiblePhases.length - 1; i++) {
      const current = visiblePhases[i];
      const next = visiblePhases[i + 1];
      
      // Check if next view's proposal starts before current view's finalization ends
      if (next.proposalStart && current.finalizationEnd && next.proposalStart < current.finalizationEnd) {
        const overlapStart = pipelineTimeToX(next.proposalStart);
        const overlapEnd = pipelineTimeToX(current.finalizationEnd);
        
        if (overlapStart !== null && overlapEnd !== null) {
          const clampedStart = Math.max(sidebarWidth, Math.min(w, overlapStart));
          const clampedEnd = Math.max(sidebarWidth, Math.min(w, overlapEnd));
          
          if (clampedStart < w && clampedEnd > sidebarWidth) {
            const y1 = topPadding + i * rowHeight + barHeight / 2;
            const y2 = topPadding + (i + 1) * rowHeight + barHeight / 2;
            
            // Draw a bracket/connection showing the overlap
            pipelineCtx.beginPath();
            pipelineCtx.moveTo(clampedStart, y2);
            pipelineCtx.lineTo(clampedStart, (y1 + y2) / 2);
            pipelineCtx.lineTo(clampedEnd, (y1 + y2) / 2);
            pipelineCtx.lineTo(clampedEnd, y1);
            pipelineCtx.stroke();
          }
        }
      }
    }
    pipelineCtx.setLineDash([]);
    pipelineCtx.restore();
  }

  function updateFooter() {
    const visibleCount = state.lastVisible.length;
    const viewRange = getVisibleViewRange();
    const timeRangeMs = Number(state.spanNs) / 1_000_000;
    
    elements.viewInfo.innerHTML = `
      <span class="info-item">
        <span class="label">Events:</span>
        <span class="value">${visibleCount.toLocaleString()} / ${state.events.length.toLocaleString()}</span>
      </span>
      <span class="info-item">
        <span class="label">Views:</span>
        <span class="value">${viewRange}</span>
      </span>
    `;
    
    elements.timeInfo.innerHTML = `
      <span class="info-item">
        <span class="label">Window:</span>
        <span class="value">${formatTime(timeRangeMs)}</span>
      </span>
      <span class="info-item">
        <span class="label">Segment:</span>
        <span class="value">${state.activeSegment + 1} / ${state.segments.length}</span>
      </span>
    `;
  }

  // ============================================================================
  // Tooltip
  // ============================================================================

  function showTooltip(ev, x, y) {
    const relTime = Number(BigInt(ev.raw.timestamp_ns) - BigInt(state.meta.min_ts_ns)) / 1_000_000;
    const rows = [
      `<div class="tooltip-header"><span class="color-dot" style="background:${getKindColor(ev.kind_tag)}"></span><span class="kind">${getKindLabel(ev.kind_tag)}</span></div>`,
      `<div class="tooltip-row"><span class="label">Node</span><span class="value">${ev.raw.node_id}</span></div>`,
      `<div class="tooltip-row"><span class="label">View</span><span class="value highlight">${ev.view ?? '-'}</span></div>`,
      ev.leader != null ? `<div class="tooltip-row"><span class="label">Leader</span><span class="value">${ev.leader}</span></div>` : '',
      `<div class="tooltip-row"><span class="label">Time</span><span class="value">+${formatTime(relTime)}</span></div>`,
      ev.payload ? `<div class="tooltip-row"><span class="label">Payload</span><span class="value">${ev.payload.slice(0, 12)}...</span></div>` : '',
      `<div class="tooltip-message">${escapeHtml(ev.raw.message.slice(0, 150))}</div>`
    ];
    
    elements.tooltip.innerHTML = rows.join('');
    elements.tooltip.classList.add('visible');
    
    const rect = elements.timeline.getBoundingClientRect();
    const tooltipRect = elements.tooltip.getBoundingClientRect();
    let left = x + 15, top = y - 10;
    if (left + tooltipRect.width > rect.width) left = x - tooltipRect.width - 15;
    if (top + tooltipRect.height > rect.height) top = y - tooltipRect.height + 10;
    elements.tooltip.style.left = `${left}px`;
    elements.tooltip.style.top = `${top}px`;
  }

  function hideTooltip() {
    elements.tooltip.classList.remove('visible');
    state.hoveredEvent = null;
  }

  // ============================================================================
  // Event Handlers
  // ============================================================================

  function findClosestEvent(x, y) {
    let closest = null, closestDist = Infinity;
    for (const p of state.lastVisible) {
      const dist = Math.hypot(p.x - x, p.y - y);
      if (dist < closestDist && dist < p.r + 8) {
        closestDist = dist;
        closest = p;
      }
    }
    return closest;
  }

  function handleMouseMove(e) {
    const rect = elements.timeline.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const closest = findClosestEvent(x, y);
    
    if (closest) {
      state.hoveredEvent = closest.event;
      elements.timeline.style.cursor = 'pointer';
      showTooltip(closest.event, x, y);
      draw();
    } else {
      if (state.hoveredEvent) {
        state.hoveredEvent = null;
        draw();
      }
      elements.timeline.style.cursor = 'default';
      hideTooltip();
    }
  }

  function handleMouseClick(e) {
    const rect = elements.timeline.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const closest = findClosestEvent(x, y);
    state.selectedEvent = closest?.event || null;
    draw();
  }

  function handleKeyDown(e) {
    const keyActions = {
      'ArrowLeft': () => pan(-0.15),
      'ArrowRight': () => pan(0.15),
      'BracketLeft': () => zoom(0.7),
      'BracketRight': () => zoom(1.4),
      'KeyR': () => { computeViewport(); draw(); },
      'Escape': () => { state.selectedEvent = null; hideTooltip(); draw(); }
    };
    
    if (keyActions[e.code]) {
      keyActions[e.code]();
    } else if (e.code.startsWith('Digit')) {
      const segIdx = parseInt(e.key) - 1;
      if (segIdx >= 0 && segIdx < state.segments.length) {
        setViewportFromSegment(segIdx);
      }
    }
  }

  function handleWheel(e) {
    e.preventDefault();
    
    if (e.ctrlKey || e.metaKey) {
      // Zoom
      const factor = e.deltaY > 0 ? 1.2 : 0.8;
      zoom(factor);
    } else {
      // Pan
      const fraction = e.deltaX !== 0 ? e.deltaX / 500 : e.deltaY / 500;
      pan(fraction);
    }
  }

  function handleMinimapClick(e) {
    const rect = elements.minimap.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const w = rect.width;
    
    const minTs = BigInt(state.meta.min_ts_ns);
    const maxTs = BigInt(state.meta.max_ts_ns);
    const totalRange = maxTs - minTs;
    
    const clickedTs = minTs + (totalRange * BigInt(Math.floor(x))) / BigInt(w);
    
    // Center viewport on clicked position
    state.centerNs = clickedTs;
    state.viewStart = state.centerNs - state.spanNs / 2n;
    state.viewEnd = state.centerNs + state.spanNs / 2n;
    
    draw();
  }

  // ============================================================================
  // Utility Functions
  // ============================================================================

  function getKind(tag) {
    return state.meta?.event_kinds?.find(k => k.tag === tag);
  }

  function getKindColor(tag) {
    return getKind(tag)?.color || defaultKindColors[tag] || '#9ca3af';
  }

  function getKindLabel(tag) {
    return getKind(tag)?.label || `Kind ${tag}`;
  }

  function getCategoryFromKindTag(tag) {
    if (tag <= 20) return 'consensus';
    if (tag <= 30) return 'marshal';
    if (tag <= 40) return 'batcher';
    if (tag <= 50) return 'broadcast';
    if (tag <= 60) return 'resolver';
    if (tag <= 80) return 'network';
    if (tag <= 90) return 'storage';
    return 'other';
  }

  function getNodeColor(nodeId) {
    const colors = ['#58a6ff', '#3fb950', '#f0883e', '#a371f7', '#f85149', '#39c5cf'];
    return colors[nodeId % colors.length];
  }

  function getVisibleViewRange() {
    const views = new Set();
    state.lastVisible.forEach(p => {
      if (p.event.view !== null && p.event.view !== undefined) {
        views.add(p.event.view);
      }
    });
    
    if (views.size === 0) return '-';
    const sorted = Array.from(views).sort((a, b) => a - b);
    if (sorted.length === 1) return `${sorted[0]}`;
    return `${sorted[0]} - ${sorted[sorted.length - 1]}`;
  }

  function formatNumber(n) {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return n.toString();
  }

  function formatTime(ms) {
    if (ms < 1) return `${(ms * 1000).toFixed(0)}µs`;
    if (ms < 1000) return `${ms.toFixed(1)}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  // ============================================================================
  // Initialization
  // ============================================================================

  function toggleViewMode() {
    const isPipelining = state.viewMode === 'standard';
    state.viewMode = isPipelining ? 'pipelining' : 'standard';
    const btn = elements.viewModeToggle;
    const appEl = document.getElementById('app');
    if (btn) {
      btn.classList.remove('attention');
      btn.classList.toggle('active', isPipelining);
      btn.setAttribute('aria-pressed', isPipelining ? 'true' : 'false');
      const iconEl = btn.querySelector('.icon');
      const labelEl = btn.querySelector('.label');
      const eyebrowEl = btn.querySelector('.eyebrow');
      const ctaEl = btn.querySelector('.cta');
      if (iconEl) iconEl.textContent = isPipelining ? '⊞' : '⟂';
      if (labelEl) labelEl.textContent = isPipelining ? 'Pipelining View' : 'Standard View';
      if (eyebrowEl) eyebrowEl.textContent = 'Visualization';
      if (ctaEl) ctaEl.textContent = isPipelining ? 'Return to standard timeline' : 'Try pipelining mode';
    }
    if (appEl) {
      appEl.classList.toggle('pipelining-mode', isPipelining);
    }
    setTimeout(() => { resize(); draw(); }, 0);
  }

  function init() {
    // Event listeners
    elements.timeline.addEventListener('mousemove', handleMouseMove);
    elements.timeline.addEventListener('mouseleave', hideTooltip);
    elements.timeline.addEventListener('click', handleMouseClick);
    elements.timeline.addEventListener('wheel', handleWheel, { passive: false });
    
    elements.minimap.addEventListener('click', handleMinimapClick);
    
    if (elements.viewModeToggle) {
      elements.viewModeToggle.addEventListener('click', toggleViewMode);
    }
    
    document.addEventListener('keydown', handleKeyDown);
    
    window.addEventListener('resize', () => {
      draw();
    });

    // Load data
    fetchData();
  }

  init();
})();
