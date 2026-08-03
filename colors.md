# Launcher Color Palette & Theme Specifications

This document lists the exact colors, opacity levels, and shadow parameters used in the launcher's default clean monotone theme, so you can adapt your system theme (e.g., in Hyprland, Waybar, kitty, or other apps) to match.

---

## 1. Backgrounds & Window Card

| Element | Color Value | Hex Equivalent | Description |
| :--- | :--- | :--- | :--- |
| **Window Background** | `rgba(22, 22, 22, 0.90)` | `#161616` at 90% opacity | Base color of the main box container. |
| **Outer Border** | `rgba(255, 255, 255, 0.08)` | `#ffffff` at 8% opacity | Thin border enclosing the window card. |
| **Box Shadow** | `rgba(0, 0, 0, 0.60)` | `#000000` at 60% opacity | Shadow parameters: `0 12px 40px` (y-offset 12px, blur 40px). |

---

## 2. Input Fields & Buttons

| Element | State | Color Value | Hex Equivalent | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Search Entry Box** | Default | `rgba(255, 255, 255, 0.03)` | `#ffffff` at 3% opacity | Search input background. |
| **Search Entry Border**| Default | `rgba(255, 255, 255, 0.05)` | `#ffffff` at 5% opacity | Thin input border. |
| **Search Entry Box** | Focused | `rgba(255, 255, 255, 0.05)` | `#ffffff` at 5% opacity | Background when active. |
| **Search Entry Border**| Focused | `rgba(255, 255, 255, 0.50)` | `#ffffff` at 50% opacity | Highlight border when typing. |
| **Buttons / Badges** | Default | `rgba(255, 255, 255, 0.05)` | `#ffffff` at 5% opacity | e.g. color picker action button. |
| **Buttons / Badges** | Hover | `rgba(255, 255, 255, 0.06)` | `#ffffff` at 6% opacity | Highlight on hover. |
| **Buttons / Badges** | Active | `rgba(255, 255, 255, 0.12)` | `#ffffff` at 12% opacity | Pressed state. |

---

## 3. Lists & Search Rows

| Element | State | Color Value | Hex Equivalent | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Row Background** | Default | `rgba(0, 0, 0, 0)` | Transparent | Default item state. |
| **Row Background** | Hover | `rgba(255, 255, 255, 0.06)` | `#ffffff` at 6% opacity | Highlight on hover. |
| **Row Background** | Selected | `rgba(255, 255, 255, 0.12)` | `#ffffff` at 12% opacity | Active selected item. |

---

## 4. Typography

| Text Role | Color Value | Hex Equivalent | Description |
| :--- | :--- | :--- | :--- |
| **Primary Headers / Titles** | `#ffffff` | `#ffffff` (100% white) | App titles, query results. |
| **Standard Row Text** | `#e0e0e0` | `#e0e0e0` (Light gray) | Unfocused item titles. |
| **Muted Descriptions** | `#888888` | `#888888` (Medium gray) | Subtitles, directories, status descriptions. |
| **Section Headings** | `rgba(255, 255, 255, 0.4)`| `#ffffff` at 40% opacity | Uppercase labels (e.g. "Deutsch", "Englisch"). |

---

## 5. Compositor Blur Specifications

If using Hyprland (or SwayFX) and configuring background blur under the `"launcher"` namespace, the recommended settings to match the window borders are:

* **Blur Mode**: Enabled
* **Ignore Alpha**: Set to `0.2`. *(This tells the compositor not to blur any pixels with opacity under 20%. This keeps the 40px outer shadow drop margins (`rgba(0, 0, 0, 0.6)`) and transparent window edges clean and un-blurred.)*
