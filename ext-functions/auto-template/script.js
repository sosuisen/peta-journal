(function() {
  let templates = [];

  // Fetch and parse template.md
  fetch('template.md')
    .then(r => r.text())
    .then(text => {
      templates = parseTemplates(text);
      console.log('[auto-template] Loaded', templates.length, 'templates');
    })
    .catch(e => console.error('[auto-template] Failed to load template.md:', e));

  function parseTemplates(text) {
    const results = [];
    // Split by --- lines (section separators)
    const sections = text.split(/^---$/m);

    for (const section of sections) {
      const trimmed = section.trim();
      if (!trimmed) continue;

      // Find the pattern line and body
      const lines = trimmed.split('\n');
      let patterns = [];
      let bodyStartIdx = -1;

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        if (line.startsWith('pattern:')) {
          const patternStr = line.substring('pattern:'.length).trim();
          // Support comma-separated patterns
          patterns = patternStr.split(',').map(p => p.trim()).filter(p => p);
        } else if (line === '' && patterns.length > 0 && bodyStartIdx === -1) {
          bodyStartIdx = i + 1;
        }
      }

      if (patterns.length > 0 && bodyStartIdx >= 0) {
        const body = lines.slice(bodyStartIdx).join('\n').trim();
        if (body) {
          results.push({ patterns, body });
        }
      }
    }
    return results;
  }

  function matchesPattern(pattern, fileName, dirName) {
    // daily: YYYY-MM-DD
    if (pattern === 'daily') {
      return /^\d{4}-\d{2}-\d{2}$/.test(fileName);
    }

    // Day of week: :Sunday, :Monday, etc.
    const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
    if (pattern.startsWith(':') && days.includes(pattern.substring(1))) {
      if (!/^\d{4}-\d{2}-\d{2}$/.test(fileName)) return false;
      const date = new Date(fileName + 'T00:00:00');
      return days[date.getDay()] === pattern.substring(1);
    }

    // weekly: YYYY-Wnn
    if (pattern === 'weekly') {
      return /^\d{4}-W\d{2}$/.test(fileName);
    }

    // monthly: YYYY-MM (but NOT YYYY-MM-DD)
    if (pattern === 'monthly') {
      return /^\d{4}-\d{2}$/.test(fileName);
    }

    // yearly: YYYY (4 digits only)
    if (pattern === 'yearly') {
      return /^\d{4}$/.test(fileName);
    }

    // page/doc: non-journal pages
    if (pattern === 'page' || pattern === 'doc') {
      return dirName.startsWith('docs');
    }

    // journal: any journal page
    if (pattern === 'journal') {
      return dirName.startsWith('journal');
    }

    // regex: /pattern/
    if (pattern.startsWith('/') && pattern.endsWith('/') && pattern.length > 2) {
      try {
        const regex = new RegExp(pattern.slice(1, -1));
        return regex.test(fileName);
      } catch (e) {
        console.warn('[auto-template] Invalid regex pattern:', pattern);
        return false;
      }
    }

    return false;
  }

  // Listen for messages from parent
  window.addEventListener('message', async function(event) {
    const data = event.data;
    if (!data || data.type !== 'PageCreate') return;

    const { pageId, fileName, dirName } = data;
    console.log('[auto-template] PageCreate:', fileName, dirName);

    // Find all matching templates
    const matched = templates.filter(t =>
      t.patterns.some(p => matchesPattern(p, fileName, dirName))
    );

    if (matched.length === 0) {
      console.log('[auto-template] No matching templates for:', fileName);
      return;
    }

    // Collect all blocks from all matched templates
    const allBlocks = [];
    for (const template of matched) {
      // Split body by blank lines into blocks (flat markdown)
      const blocks = template.body.split(/\n\n+/);
      for (const blockBody of blocks) {
        const trimmed = blockBody.trim();
        if (trimmed) {
          allBlocks.push(trimmed);
        }
      }
    }

    // Send CreateBlock messages sequentially, waiting for response
    let lastBlockId = '';
    for (const body of allBlocks) {
      lastBlockId = await createBlock(pageId, lastBlockId, 0, body);
    }

    console.log('[auto-template] Created', allBlocks.length, 'blocks for:', fileName);
  });

  function createBlock(pageId, afterBlockId, level, body) {
    return new Promise((resolve) => {
      const requestId = 'req-' + Date.now() + '-' + Math.random().toString(36).substring(2, 8);

      function onResponse(event) {
        const data = event.data;
        if (!data || data.requestId !== requestId) return;
        window.removeEventListener('message', onResponse);
        if (data.type === 'CreateBlockResult') {
          resolve(data.blockId);
        } else {
          console.error('[auto-template] CreateBlock failed:', data.error);
          resolve(''); // Continue even on error
        }
      }

      window.addEventListener('message', onResponse);

      parent.postMessage({
        type: 'CreateBlock',
        pageId,
        afterBlockId,
        level,
        body,
        requestId
      }, '*');

      // Timeout fallback
      setTimeout(() => {
        window.removeEventListener('message', onResponse);
        resolve('');
      }, 5000);
    });
  }
})();
