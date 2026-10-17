(function() {
  let templates = [];

  // Fetch and parse template.md. A PageCreate that arrives first waits for this.
  const templatesLoaded = fetch('template.md')
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

  /**
   * Split flat markdown text into items with the server-side parser, so the template
   * is cut at the same block boundaries as a markdown file. The iframe is served by the
   * journal server, so the API is same-origin.
   * A GFM task prefix (`- [ ] ` / `- [x] `) stays in the body; the server detects the task from it.
   * Returns [{ level, body }].
   */
  async function splitFlatMarkdown(text) {
    const resp = await fetch('/api/items/parse', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ markdown: text.replace(/\r\n/g, '\n') })
    });
    if (!resp.ok) {
      throw new Error('HTTP ' + resp.status);
    }
    const result = await resp.json();
    return result.items.map(item => ({ level: item.level, body: item.body }));
  }

  function matchesPattern(pattern, fileName, dirName, pageType) {
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
      return pageType === 'doc';
    }

    // journal: any journal page
    if (pattern === 'journal') {
      return pageType === 'journal';
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

    const { pageId, fileName, dirName, pageType } = data;
    console.log('[auto-template] PageCreate:', fileName, pageType);
    await templatesLoaded;

    // Find all matching templates
    const matched = templates.filter(t =>
      t.patterns.some(p => matchesPattern(p, fileName, dirName, pageType))
    );

    if (matched.length === 0) {
      console.log('[auto-template] No matching templates for:', fileName);
      return;
    }

    // Collect all items from all matched templates
    const allItems = [];
    for (const template of matched) {
      try {
        allItems.push(...await splitFlatMarkdown(template.body));
      } catch (e) {
        console.error('[auto-template] Failed to parse template:', e);
      }
    }

    // Send CreateItem messages sequentially, waiting for response
    let lastItemId = '';
    for (const item of allItems) {
      lastItemId = await createItem(pageId, lastItemId, item.level, item.body);
    }

    console.log('[auto-template] Created', allItems.length, 'items for:', fileName);
  });

  function createItem(pageId, afterItemId, level, body) {
    return new Promise((resolve) => {
      const requestId = 'req-' + Date.now() + '-' + Math.random().toString(36).substring(2, 8);

      function onResponse(event) {
        const data = event.data;
        if (!data || data.requestId !== requestId) return;
        window.removeEventListener('message', onResponse);
        if (data.type === 'CreateItemResult') {
          resolve(data.itemId);
        } else {
          console.error('[auto-template] CreateItem failed:', data.error);
          resolve(''); // Continue even on error
        }
      }

      window.addEventListener('message', onResponse);

      parent.postMessage({
        type: 'CreateItem',
        pageId,
        afterItemId,
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
