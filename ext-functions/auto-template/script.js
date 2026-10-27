(function() {
  let templates = [];

  // Fetch and parse every .md file of this function, in file name order.
  // A PageCreate that arrives first waits for this.
  const templatesLoaded = loadTemplates()
    .then(() => console.log('[auto-template] Loaded', templates.length, 'templates'))
    .catch(e => console.error('[auto-template] Failed to load templates:', e));

  async function loadTemplates() {
    // The server lists the files of this function at its directory URL without the trailing '/'
    const dirUrl = new URL('.', location.href).pathname.replace(/\/$/, '');
    const resp = await fetch(dirUrl);
    if (!resp.ok) {
      throw new Error('HTTP ' + resp.status);
    }
    const names = (await resp.json()).filter(name => name.toLowerCase().endsWith('.md'));
    for (const name of names) {
      // A broken file should not take the templates of the other files with it
      try {
        const text = await fetch(encodeURIComponent(name)).then(r => r.text());
        templates.push(...parseTemplates(text));
      } catch (e) {
        console.error('[auto-template] Failed to load ' + name + ':', e);
      }
    }
  }

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

      // Parse pattern and tag from frontmatter
      let patterns = [];
      let tags = [];
      for (const line of frontmatter.split('\n')) {
        const trimmed = line.trim();
        if (trimmed.startsWith('pattern:')) {
          patterns = splitList(trimmed.substring('pattern:'.length));
        }
        if (trimmed.startsWith('tag:')) {
          // A '#' typed out of habit is not part of the tag, as in the tags property
          tags = splitList(trimmed.substring('tag:'.length))
            .map(t => t.replace(/^#+/, '').trim())
            .filter(t => t);
        }
      }

      if ((patterns.length > 0 || tags.length > 0) && markdown) {
        results.push({ patterns, tags, body: markdown });
      }
    }
    return results;
  }

  function splitList(text) {
    return text.split(',').map(p => p.trim()).filter(p => p);
  }

  /**
   * Split flat markdown text into items with the server-side parser, so the template
   * is cut at the same block boundaries as a markdown file. The iframe is served by the
   * journal server, so the API is same-origin.
   * A GFM task prefix (`- [ ] ` / `- [x] `) stays in the body; the server detects the task from it.
   * Returns [{ level, body, transclude }]; transclude is the id of the item to transclude, or null.
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
    return result.items.map(item => ({
      level: item.level,
      body: item.body,
      transclude: (item.properties && item.properties.transclude) || null
    }));
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
    if (!data) return;
    if (data.type === 'PageCreate') {
      await onPageCreate(data);
    } else if (data.type === 'PageTagAdd') {
      await onPageTagAdd(data);
    }
  });

  async function onPageCreate(data) {
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

    const count = await insertTemplates(pageId, matched, 'first');
    console.log('[auto-template] Created', count, 'items for:', fileName);
  }

  // The templates of each added tag go to the end of the page, once the user agrees
  async function onPageTagAdd(data) {
    const { pageId, fileName, tags } = data;
    console.log('[auto-template] PageTagAdd:', fileName, tags);
    await templatesLoaded;

    for (const tag of tags) {
      const matched = templates.filter(t => t.tags.includes(tag));
      if (matched.length === 0) continue;
      const ok = await confirm(`Insert the template of the tag "#${tag}" at the end of this page?`);
      if (!ok) continue;
      const count = await insertTemplates(pageId, matched, 'last');
      console.log('[auto-template] Created', count, 'items for tag:', tag);
    }
  }

  /**
   * Insert the items of the templates in order. The first item goes to `position`
   * ('first' or 'last'); each next one goes after the previous one.
   * Returns the number of items.
   */
  async function insertTemplates(pageId, matched, position) {
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
      // A failed item gives no id, so the next one falls back to `position`
      const where = lastItemId ? 'after:' + lastItemId : position;
      lastItemId = await createItem(pageId, where, item.level, item.body, item.transclude);
    }
    return allItems.length;
  }

  function newRequestId() {
    return 'req-' + Date.now() + '-' + Math.random().toString(36).substring(2, 8);
  }

  // window.confirm is blocked in a hidden cross-origin iframe, so the app asks instead
  function confirm(message) {
    return new Promise((resolve) => {
      const requestId = newRequestId();

      function onResponse(event) {
        const data = event.data;
        if (!data || data.requestId !== requestId || data.type !== 'ConfirmResult') return;
        window.removeEventListener('message', onResponse);
        resolve(data.ok === true);
      }

      window.addEventListener('message', onResponse);
      // No timeout: the user may take any time to answer the dialog
      parent.postMessage({ type: 'Confirm', message, requestId }, '*');
    });
  }

  function createItem(pageId, position, level, body, transclude) {
    return new Promise((resolve) => {
      const requestId = newRequestId();

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
        position,
        level,
        body,
        transclude,
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
