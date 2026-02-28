// MC-RS Documentation — Search
(function() {
  'use strict';

  var searchIndex = window.SEARCH_DATA || [];
  var searchInput = document.querySelector('.search-input');
  var searchResults = document.querySelector('.search-results');

  // Determine base path (are we in pages/ or root?)
  var isSubpage = window.location.pathname.indexOf('/pages/') !== -1;
  var basePath = isSubpage ? '../' : './';

  // Keyboard shortcut: / to focus search
  document.addEventListener('keydown', function(e) {
    if (e.key === '/' && document.activeElement !== searchInput) {
      e.preventDefault();
      if (searchInput) searchInput.focus();
    }
    if (e.key === 'Escape') {
      if (searchResults) searchResults.classList.remove('active');
      if (searchInput) searchInput.blur();
    }
  });

  if (searchInput) {
    searchInput.addEventListener('input', function() {
      var query = this.value.trim().toLowerCase();
      if (query.length < 2) {
        searchResults.classList.remove('active');
        return;
      }
      var results = performSearch(query);
      renderResults(results, query);
    });

    searchInput.addEventListener('focus', function() {
      if (this.value.trim().length >= 2) {
        searchResults.classList.add('active');
      }
    });
  }

  // Close on click outside
  document.addEventListener('click', function(e) {
    if (searchResults && !searchResults.contains(e.target) && e.target !== searchInput) {
      searchResults.classList.remove('active');
    }
  });

  function performSearch(query) {
    var words = query.split(/\s+/);
    var scored = [];

    for (var i = 0; i < searchIndex.length; i++) {
      var item = searchIndex[i];
      var titleLower = item.title.toLowerCase();
      var contentLower = item.content.toLowerCase();
      var score = 0;

      for (var j = 0; j < words.length; j++) {
        var w = words[j];
        if (titleLower.indexOf(w) !== -1) score += 10;
        if (contentLower.indexOf(w) !== -1) score += 1;
      }

      if (score > 0) {
        scored.push({ item: item, score: score });
      }
    }

    scored.sort(function(a, b) { return b.score - a.score; });
    return scored.slice(0, 8);
  }

  function renderResults(results, query) {
    if (!searchResults) return;

    if (results.length === 0) {
      searchResults.innerHTML = '<div class="search-no-results">No results for "' + escapeHtml(query) + '"</div>';
      searchResults.classList.add('active');
      return;
    }

    var html = '';
    for (var i = 0; i < results.length; i++) {
      var item = results[i].item;
      var url = basePath + item.url;
      var excerpt = getExcerpt(item.content, query);

      html += '<a class="search-result-item" href="' + url + '">';
      html += '<div class="search-result-title">' + escapeHtml(item.title) + '</div>';
      html += '<div class="search-result-section">' + escapeHtml(item.section) + '</div>';
      if (excerpt) {
        html += '<div class="search-result-excerpt">' + excerpt + '</div>';
      }
      html += '</a>';
    }

    searchResults.innerHTML = html;
    searchResults.classList.add('active');
  }

  function getExcerpt(content, query) {
    var lower = content.toLowerCase();
    var idx = lower.indexOf(query.split(/\s+/)[0]);
    if (idx === -1) return '';

    var start = Math.max(0, idx - 40);
    var end = Math.min(content.length, idx + 80);
    var excerpt = (start > 0 ? '...' : '') + content.substring(start, end) + (end < content.length ? '...' : '');

    // Highlight matches
    var words = query.split(/\s+/);
    for (var i = 0; i < words.length; i++) {
      var re = new RegExp('(' + escapeRegex(words[i]) + ')', 'gi');
      excerpt = escapeHtml(excerpt).replace(re, '<mark>$1</mark>');
    }
    return excerpt;
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }
})();
