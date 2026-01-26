# Media Server UI Implementation Plan

## Overview
Implement a GTK-inspired Media Server UI similar to the provided example, using the existing typography color system. The UI will be built using Server-Sent Events (SSE) with Datastar.

## Color Mapping (Typography Variables)

The existing typography system provides these color variables that we'll use:

### Background Colors
- `--window-bg-color`: Main container background (#222126 dark, #fafafb light)
- `--view-bg-color`: Body content area (#1d1d20 dark, #ffffff light)
- `--sidebar-bg-color`: Sidebar (#2e2e32 dark, #ebebed light)
- `--headerbar-bg-color`: Header (#2e2e32 dark, #ffffff light)
- `--card-bg-color`: Cards (rgba(255,255,255,0.08) dark, #ffffff light)

### Text Colors
- `--window-fg-color`: Main text (#ffffff dark, rgba(0,0,6,0.8) light)
- `--view-fg-color`: Body text (#ffffff dark, rgba(0,0,6,0.8) light)
- `--sidebar-fg-color`: Sidebar text (#ffffff dark, rgba(0,0,6,0.8) light)
- `--headerbar-fg-color`: Header text (#ffffff dark, rgba(0,0,6,0.8) light)

### Accent Colors
- `--accent-color`: Primary accent (blue-2 for dark: #62a0ea, blue-4 for light: #1c71d8)
- `--accent-bg-color`: Background accent (blue-3: #3584e4)
- `--accent-fg-color`: Accent foreground (#ffffff)

### Utility Colors
- `--borders`: Border color (color-mix with 15% opacity)
- `--hover-color`: Hover state (color-mix with 7% opacity)
- `--active-color`: Active state (color-mix with 16% opacity)

## Component Updates

### 1. Sidebar (`src/components/layout/sidebar.rs`)

**HTML Structure:**
```html
<nav id="sidebar">
    <div class="sidebar-header">
        <button onclick="toggleSidebar()" class="btn btn-ghost">
            <i class="ph ph-list text-xl"></i>
        </button>
    </div>
    <ul class="nav-list">
        <li class="nav-item active">
            <div class="nav-icon"><i class="ph ph-squares-four"></i></div>
            <span>Dashboard</span>
        </li>
        <!-- More nav items... -->
    </ul>
    <div class="sidebar-footer">
        <!-- Settings item -->
    </div>
</nav>
```

### 2. Header (`src/components/layout/header.rs`)

**HTML Structure:**
```html
<header id="header">
    <div class="header-spacer"></div>
    <div class="header-title">Dashboard</div>
    <div class="header-controls">
        <button class="btn btn-ghost"><i class="ph ph-magnifying-glass"></i></button>
        <div class="header-separator"></div>
        <button class="btn btn-ghost"><i class="ph ph-bell"></i></button>
        <button class="btn btn-ghost"><i class="ph ph-user"></i></button>
    </div>
</header>
```

### 3. Body (`src/components/layout/body.rs`)

**HTML Structure:**
```html
<div id="body">
    <!-- Server Status Card -->
    <div class="gtk-card">
        <div class="card-header">
            <div>
                <div class="card-title">Serverstatus</div>
                <p class="card-subtitle">Alle Systeme laufen normal.</p>
            </div>
            <label class="toggle-switch">
                <input type="checkbox" checked>
                <span class="slider"></span>
            </label>
        </div>
        <div class="stats-grid">
            <!-- Stats boxes -->
        </div>
    </div>

    <!-- Recent Media Card -->
    <div class="gtk-card">
        <!-- Media list -->
    </div>

    <!-- Network/Storage Card -->
    <div class="gtk-card">
        <!-- Progress bars -->
    </div>
</div>
```

## CSS Implementation (`public/pages/home.css`)

### Layout Variables
```css
:root {
    --radius-window: 16px;
    --radius-card: 12px;
    --radius-btn: 6px;
    --sidebar-width: 260px;
    --sidebar-width-collapsed: 64px;
    --header-height: 64px;
}
```

### Main Container
- Centered, max-width 1400px, 90vh height
- Background: `--window-bg-color`
- Border: `--borders`
- Border-radius: `--radius-window`
- Box-shadow for window effect

### Sidebar
- Absolute positioning, left side
- Background: `--sidebar-bg-color`
- Width: `--sidebar-width`
- Transition for collapsed state
- Navigation items with hover/active states using `--hover-color` and `--active-color`

### Header
- Height: `--header-height`
- Background: `--headerbar-bg-color`
- Text: `--headerbar-fg-color`
- Shadow: `--headerbar-shade-color`
- Margin-left to account for sidebar

### Body
- Background: `--view-bg-color`
- Padding: 24px
- Overflow-y: auto
- Margin-left to account for sidebar

### Cards
- Background: `--card-bg-color`
- Border: 1px solid `--borders`
- Border-radius: `--radius-card`
- Padding: 24px

### Buttons
- `.btn-primary`: Background `--accent-bg-color`, text `--accent-fg-color`
- `.btn-ghost`: Transparent, hover background `--hover-color`

### Toggle Switch
- Track: dark gray (#2d2d2d)
- Active: `--accent-bg-color`

### Progress Bars
- Track: dark gray
- Fill: `--accent-color` or white

### Scrollbar
- Custom styling matching the theme

## External Dependencies
- Phosphor Icons: `<script src="https://unpkg.com/@phosphor-icons/web"></script>`
- Datastar: Already included in home.rs template

## JavaScript
- `toggleSidebar()` function to toggle `.collapsed` class on `#main-container`

## Implementation Order

1. Update Sidebar component HTML template
2. Update Header component HTML template
3. Update Body component HTML template with dashboard content
4. Add base layout CSS (container, body reset)
5. Add sidebar CSS (navigation, collapsed state)
6. Add header CSS (shadow, controls)
7. Add card and content CSS (stats, media list)
8. Add component CSS (buttons, toggles, progress bars)
9. Add utility CSS (scrollbar, transitions)
