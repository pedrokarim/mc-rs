// MC-RS Documentation — Navigation
(function() {
  'use strict';

  // Mobile sidebar toggle
  const hamburger = document.querySelector('.hamburger');
  const sidebar = document.querySelector('.sidebar');
  const overlay = document.querySelector('.sidebar-overlay');

  if (hamburger && sidebar) {
    hamburger.addEventListener('click', function() {
      sidebar.classList.toggle('open');
      if (overlay) overlay.classList.toggle('active');
    });
  }

  if (overlay) {
    overlay.addEventListener('click', function() {
      sidebar.classList.remove('open');
      overlay.classList.remove('active');
    });
  }

  // Mark active page in sidebar
  const currentPath = window.location.pathname;
  const isHome = currentPath.endsWith('/') || currentPath.endsWith('/index.html');
  const links = document.querySelectorAll('.sidebar-link');
  links.forEach(function(link) {
    const href = link.getAttribute('href');
    if (!href || href.startsWith('http')) return;
    // Home link detection
    if (href === './' || href === '../' || href === '../index.html' || href === './index.html') {
      if (isHome) link.classList.add('active');
      return;
    }
    // Sub-page: match filename
    var hrefFile = href.split('/').pop();
    var pathFile = currentPath.split('/').pop();
    if (hrefFile && pathFile && hrefFile === pathFile) {
      link.classList.add('active');
    }
  });
})();
