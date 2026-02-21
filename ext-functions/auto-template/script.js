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
    // Split by --- lines into alternating frontmatter/markdown sections
    const sections = text.split(/^---\r?$/m);

    // sections[0] is before the first ---, should be empty
    // Then: frontmatter, markdown, frontmatter, markdown, ...
    for (let i = 1; i < sections.length; i += 2) {
      const frontmatter = sections[i].trim();
      const markdown = (i + 1 < sections.length) ? sections[i + 1].trim() : '';

      if (!frontmatter) continue;

      // Parse pattern from frontmatter
      let patterns = [];
      for (const line of frontmatter.split('\n')) {
        const trimmed = line.trim();
        if (trimmed.startsWith('pattern:')) {
          const patternStr = trimmed.substring('pattern:'.length).trim();
          patterns = patternStr.split(',').map(p => p.trim()).filter(p => p);
        }
      }

      if (patterns.length > 0 && markdown) {
        results.push({ patterns, body: markdown });
      }
    }
    return results;
  }

  function parseBlock(text) {
    let level = 1;
    let status = 'normal';
    const bodyLines = [];

    for (const line of text.split('\n')) {
      const levelMatch = line.match(/^<!-- level=(\d+) -->\s*$/);
      if (levelMatch) {
        level = parseInt(levelMatch[1], 10);
        continue;
      }
      // Skip other metadata comments
      if (/^<!-- \w+=.*? -->\s*$/.test(line)) continue;

      bodyLines.push(line);
    }

    let body = bodyLines.join('\n').trim();

    // Detect GFM task prefix
    if (body.startsWith('- [ ] ')) {
      status = 'todo';
      body = body.substring(6);
    } else if (body.startsWith('- [x] ')) {
      status = 'done';
      body = body.substring(6);
    }

    // Whitespace-only body → empty
    if (body.length > 0 && body.trim() === '') body = '';

    return { level, body, status };
  }

  function matchesPattern(pattern, fileName, dirName) {
    // daily: YYYY-MM-DD
    if (pattern === 'daily') {
      return /^\d{4}-\d{2}-\d{2}$/.test(fileName);
    }

    // Day of week: Sunday, Monday, etc.
    const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
    if (days.includes(pattern)) {
      if (!/^\d{4}-\d{2}-\d{2}$/.test(fileName)) return false;
      const date = new Date(fileName + 'T00:00:00');
      return days[date.getDay()] === pattern;
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
      const blocks = template.body.split(/(?:\r?\n){2,}/);
      for (const blockBody of blocks) {
        const trimmed = blockBody.trim();
        if (!trimmed) continue;
        const block = parseBlock(trimmed);
        allBlocks.push(block);
      }
    }

    // Send CreateBlock messages sequentially, waiting for response
    let lastBlockId = '';
    for (const block of allBlocks) {
      lastBlockId = await createBlock(pageId, lastBlockId, block.level, block.body, block.status);
    }

    console.log('[auto-template] Created', allBlocks.length, 'blocks for:', fileName);
  });

  function createBlock(pageId, afterBlockId, level, body, status) {
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
        status,
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
