// Native popovers own visibility and light dismissal. Delegation survives route
// changes without retaining removed rows; document::Script loads this asset once.
(() => {
  const selector = '.menu-popover[popover]';
  const anchors = new WeakMap();
  const triggerFor = menu => document.getElementById(menu.dataset.popoverTrigger || `${menu.id}-trigger`);
  const isCalendar = menu => menu.dataset.calendar === 'true';
  const itemsFor = menu => [...menu.querySelectorAll('[role="menuitem"]:not(:disabled)')];
  const focus = item => item?.focus({ preventScroll: true });
  function focusItem(menu, item) {
    if (!item) return;
    focus(item);
    const box = menu.getBoundingClientRect(), bounds = item.getBoundingClientRect();
    if (bounds.bottom > box.bottom) menu.scrollTop += bounds.bottom - box.bottom;
    else if (bounds.top < box.top) menu.scrollTop -= box.top - bounds.top;
  }
  function close(menu, restoreFocus = false) {
    menu.hidePopover();
    if (restoreFocus) focus(triggerFor(menu));
  }

  document.addEventListener('beforetoggle', event => {
    const menu = event.target;
    if (!menu.matches(selector)) return;
    const trigger = triggerFor(menu);
    trigger.setAttribute('aria-expanded', String(event.newState === 'open'));
    if (event.newState !== 'open') {
      if (isCalendar(menu)) {
        for (const property of ['left', 'top', 'max-height']) menu.style.removeProperty(property);
      }
      return;
    }
    const anchor = trigger.getBoundingClientRect();
    anchors.set(menu, anchor);
    const gap = 4, edge = 8;
    // Measure before opening without painting or scrolling the containing table.
    menu.style.display = 'block';
    menu.style.maxHeight = '';
    const width = menu.offsetWidth, height = menu.offsetHeight;
    const below = Math.max(0, innerHeight - anchor.bottom - gap - edge);
    const above = Math.max(0, anchor.top - gap - edge);
    const upward = height > below && above > below;
    menu.style.maxHeight = `${Math.max(1, upward ? above : below)}px`;
    const left = menu.classList.contains('menu-popover-right') ? anchor.right - width : anchor.left;
    menu.style.left = `${Math.max(edge, Math.min(left, innerWidth - width - edge))}px`;
    menu.style.top = `${Math.max(edge, upward ? anchor.top - gap - menu.offsetHeight : anchor.bottom + gap)}px`;
    menu.style.removeProperty('display');
  }, true);

  document.addEventListener('toggle', event => {
    const menu = event.target;
    if (!menu.matches(selector) || !menu.matches(':popover-open')) return;
    if (isCalendar(menu)) {
      focusItem(menu, menu.querySelector('.dp-day.picked') || menu.querySelector('.dp-day'));
      return;
    }
    const items = itemsFor(menu);
    focusItem(menu, menu.dataset.last === 'true' ? items.at(-1) : items[0]);
    delete menu.dataset.last;
  }, true);

  document.addEventListener('keydown', event => {
    const trigger = event.target.closest('button[popovertarget][aria-haspopup="menu"]');
    if (trigger && !trigger.disabled && ['ArrowDown', 'ArrowUp'].includes(event.key)) {
      event.preventDefault();
      const menu = document.getElementById(trigger.getAttribute('popovertarget'));
      menu.dataset.last = String(event.key === 'ArrowUp');
      menu.showPopover();
      return;
    }
    const menu = event.target.closest(selector);
    if (!menu?.matches(':popover-open')) return;
    // Calendar buttons retain native Tab/Shift+Tab traversal, unlike menu items.
    if (isCalendar(menu) && event.key !== 'Escape') return;
    if (event.key === 'Escape' || event.key === 'Tab') {
      // Keep Escape from also closing an enclosing mobile navigation panel.
      event.stopPropagation();
      if (event.key === 'Escape') event.preventDefault();
      close(menu, true);
      return;
    }
    const items = itemsFor(menu), index = items.indexOf(document.activeElement);
    let next;
    if (event.key === 'ArrowDown') next = items[(index + 1) % items.length];
    else if (event.key === 'ArrowUp') next = items[(index + items.length - 1) % items.length];
    else if (event.key === 'Home') next = items[0];
    else if (event.key === 'End') next = items.at(-1);
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey && event.key !== ' ') {
      next = [...items.slice(index + 1), ...items.slice(0, index + 1)]
        .find(item => item.textContent.trim().toLocaleLowerCase().startsWith(event.key.toLocaleLowerCase()));
    }
    if (next) {
      event.preventDefault();
      focusItem(menu, next);
    }
  }, true);

  document.addEventListener('click', event => {
    const clear = event.target.closest('[data-clear-date]');
    if (clear && !clear.matches(':disabled')) focus(document.getElementById(clear.dataset.clearDate));
    const calendarButton = event.target.closest('[data-calendar="true"] .dp button:not(.dp-nav):not(:disabled)');
    const calendar = calendarButton?.closest(selector);
    if (calendar?.matches(':popover-open')) close(calendar, true);
    const item = event.target.closest('[role="menuitem"]:not(:disabled)');
    const menu = item?.closest(selector);
    if (menu?.matches(':popover-open')) close(menu, true);
  }, true);

  document.addEventListener('focusin', event => {
    for (const calendar of document.querySelectorAll(`${selector}[data-calendar="true"]:popover-open`)) {
      if (!calendar.contains(event.target) && event.target !== triggerFor(calendar)) close(calendar);
    }
  });

  // Dismiss when the anchor moves. Scrolling within a short menu stays usable.
  document.addEventListener('scroll', event => {
    for (const menu of document.querySelectorAll(`${selector}:popover-open`)) {
      if (menu.contains(event.target)) continue;
      const previous = anchors.get(menu), current = triggerFor(menu).getBoundingClientRect();
      // A scroll that revealed the trigger may still be queued when it opens.
      if (previous.x !== current.x || previous.y !== current.y)
        close(menu, menu.contains(document.activeElement));
    }
  }, true);
  window.addEventListener('resize', () => {
    for (const menu of document.querySelectorAll(`${selector}:popover-open`))
      close(menu, menu.contains(document.activeElement));
  });
})();
