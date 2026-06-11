---
name: Atomic Design
description: This skill should be used when the user or an agent is building UI components, designing the design system, or understanding how UI components are organized in a Claude-Factory managed project. It explains Brad Frost's Atomic Design methodology with the Claude-Factory extension adding a 'quarks' level below atoms.
version: 1.0.0
---

Claude-Factory uses Atomic Design with one extension: **quarks** below atoms.

## The levels (smallest to largest)

**Quarks**: The most primitive design tokens — colors, type scales, spacing units, border radii, shadows. Not components; just named values. "Primary blue", "spacing-4", "font-size-body". Quarks have no structure; they are referenced by atoms.

**Atoms**: The smallest functional UI elements that cannot be broken down further. A button, an input field, a label, an icon, a single-line text display. An atom uses quarks for its visual properties but has no composition of other atoms.

**Molecules**: Simple groups of atoms that work together as a unit. A labeled input field (label atom + input atom). A search bar (input atom + button atom). Molecules do one thing.

**Organisms**: Relatively complex UI sections composed of groups of molecules and/or atoms. A navigation bar (logo atom + nav-link molecules). A product card (image atom + title atom + price molecule + add-to-cart button atom). Organisms are distinct sections of an interface.

**Templates**: Page-level objects that place components into a layout. No real content — use placeholder data. Show the structure and relationships. Define the content areas and their arrangement.

**Pages**: Specific instances of templates with real content. This is what the user actually sees. Pages test whether the template works with real content and define the route-level UI.

## Vertical-slice alignment

Each vertical slice owns its screen-level UI components (the template/page instances for that slice's views). The design system (quarks through organisms) lives in the platform layer — it is shared infrastructure. A slice uses the design system's components but does not modify them.

See `references/` for how to document components at each level and how to cross-check the design system against the event model's screens.
