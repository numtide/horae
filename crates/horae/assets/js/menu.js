// Native popovers own visibility and light dismissal. Delegation survives route
// changes without retaining removed rows; document::Script loads this asset once.
(() => {
  const selector = '.menu-popover[popover]';
  const anchors = new WeakMap();
  const triggerFor = menu => document.getElementById(menu.dataset.popoverTrigger || `${menu.id}-trigger`);
  const isCalendar = menu => menu.dataset.calendar === 'true';
  const isSelect = menu => menu.dataset.select === 'true';
  const itemsFor = menu => [...menu.querySelectorAll(isSelect(menu) ? '[role="option"]:not(:disabled)' : '[role="menuitem"]:not(:disabled)')];
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

  function position(menu) {
    const trigger = triggerFor(menu);
    const scrollTop = menu.scrollTop;
    const anchor = trigger.getBoundingClientRect();
    const scrollSizes = new Map();
    for (let parent = trigger.parentElement; parent; parent = parent.parentElement)
      scrollSizes.set(parent, {
        width: parent.scrollWidth, height: parent.scrollHeight,
        left: parent.scrollLeft, top: parent.scrollTop,
      });
    anchors.set(menu, { rect: anchor, scrollSizes });
    const gap = 4, edge = 8;
    // Measure before opening without painting or scrolling the containing table.
    menu.style.display = 'block';
    menu.style.maxHeight = '';
    if (isSelect(menu)) menu.style.width = `${anchor.width}px`;
    const width = menu.offsetWidth, height = menu.offsetHeight;
    const below = Math.max(0, innerHeight - anchor.bottom - gap - edge);
    const above = Math.max(0, anchor.top - gap - edge);
    const upward = height > below && above > below;
    menu.style.maxHeight = `${Math.max(1, upward ? above : below)}px`;
    const left = menu.classList.contains('menu-popover-right') ? anchor.right - width : anchor.left;
    menu.style.left = `${Math.max(edge, Math.min(left, innerWidth - width - edge))}px`;
    menu.style.top = `${Math.max(edge, upward ? anchor.top - gap - menu.offsetHeight : anchor.bottom + gap)}px`;
    menu.style.removeProperty('display');
    menu.scrollTop = scrollTop;
  }

  // Remote search changes the panel height after opening, including above its
  // trigger on short screens. Observe only opt-in form selectors while open.
  const resize = new ResizeObserver(entries => {
    for (const { target } of entries) {
      if (!target.isConnected || !target.matches(':popover-open')) {
        resize.unobserve(target);
        continue;
      }
      position(target);
      if (target.contains(document.activeElement)) focusItem(target, document.activeElement);
    }
  });

  document.addEventListener('beforetoggle', event => {
    const menu = event.target;
    if (!menu.matches(selector)) return;
    triggerFor(menu).setAttribute('aria-expanded', String(event.newState === 'open'));
    if (event.newState !== 'open') {
      resize.unobserve(menu);
      if (isCalendar(menu) || isSelect(menu)) {
        for (const property of ['left', 'top', 'max-height', 'width']) menu.style.removeProperty(property);
      }
      return;
    }
    position(menu);
  }, true);

  document.addEventListener('toggle', event => {
    const menu = event.target;
    if (!menu.matches(selector) || !menu.matches(':popover-open')) return;
    if (isCalendar(menu)) {
      focusItem(menu, menu.querySelector('.dp-day.picked') || menu.querySelector('.dp-day'));
      return;
    }
    const items = itemsFor(menu);
    if (isSelect(menu)) {
      focusItem(menu, menu.querySelector('input') || items.find(item => item.getAttribute('aria-selected') === 'true') || items[0]);
      resize.observe(menu);
      return;
    }
    focusItem(menu, menu.dataset.last === 'true' ? items.at(-1) : items[0]);
    delete menu.dataset.last;
  }, true);

  document.addEventListener('keydown', event => {
    const trigger = event.target.closest('button[popovertarget][aria-haspopup="menu"], button[data-select-trigger]');
    if (trigger && !trigger.disabled && ['ArrowDown', 'ArrowUp'].includes(event.key)) {
      event.preventDefault();
      const menu = document.getElementById(trigger.getAttribute('popovertarget'));
      if (isSelect(menu)) {
        if (!menu.matches(':popover-open')) trigger.click();
        else focusItem(menu, itemsFor(menu)[0]);
        return;
      }
      menu.dataset.last = String(event.key === 'ArrowUp');
      menu.showPopover();
      return;
    }
    const menu = event.target.closest(selector);
    if (!menu?.matches(':popover-open')) return;
    // Calendar buttons retain native Tab/Shift+Tab traversal, unlike menu items.
    if (isCalendar(menu) && event.key !== 'Escape') return;
    if (isSelect(menu) && (event.key === 'Tab' || event.isComposing)) return;
    if (event.key === 'Escape' || event.key === 'Tab') {
      // Keep Escape from also closing an enclosing mobile navigation panel.
      event.stopPropagation();
      if (event.key === 'Escape') event.preventDefault();
      close(menu, true);
      return;
    }
    const items = itemsFor(menu), index = items.indexOf(document.activeElement);
    // Search text keeps native editing keys; arrows enter the result list.
    if (isSelect(menu) && event.target.matches('input') && !['ArrowDown', 'ArrowUp'].includes(event.key)) return;
    let next;
    if (event.key === 'ArrowDown') next = items[(index + 1) % items.length];
    else if (event.key === 'ArrowUp') next = isSelect(menu) && index < 0 ? items.at(-1) : items[(index + items.length - 1) % items.length];
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
    const item = event.target.closest('[role="menuitem"]:not(:disabled), [data-select="true"] [role="option"]:not(:disabled)');
    const menu = item?.closest(selector);
    if (menu?.matches(':popover-open')) close(menu, true);
  }, true);

  document.addEventListener('focusin', event => {
    for (const panel of document.querySelectorAll(`${selector}:popover-open`)) {
      if ((isCalendar(panel) || isSelect(panel)) && !panel.contains(event.target) && event.target !== triggerFor(panel)) close(panel);
    }
  });

  // Dismiss when the anchor moves. Scrolling within a short menu stays usable.
  document.addEventListener('scroll', event => {
    for (const menu of document.querySelectorAll(`${selector}:popover-open`)) {
      if (menu.contains(event.target)) continue;
      const previous = anchors.get(menu), current = triggerFor(menu).getBoundingClientRect();
      const scroller = event.target === document ? document.scrollingElement : event.target;
      const size = previous.scrollSizes.get(scroller);
      // Font/data reflow can clamp a scroll offset without a deliberate scroll.
      // Only retain it for that exact clamp, not a coincident deliberate scroll.
      if (size && (size.width > scroller.scrollWidth || size.height > scroller.scrollHeight)
        && scroller.scrollLeft === Math.min(size.left, scroller.scrollWidth - scroller.clientWidth)
        && scroller.scrollTop === Math.min(size.top, scroller.scrollHeight - scroller.clientHeight)) {
        position(menu);
        continue;
      }
      // A scroll that revealed the trigger may still be queued when it opens.
      if (previous.rect.x !== current.x || previous.rect.y !== current.y)
        close(menu, menu.contains(document.activeElement));
    }
  }, true);
  window.addEventListener('resize', () => {
    for (const menu of document.querySelectorAll(`${selector}:popover-open`))
      close(menu, menu.contains(document.activeElement));
  });
})();
