#!/usr/bin/env python3
"""
scripts/verify_icon_contrast.py

Computes WCAG 2.1 relative luminance and contrast ratios between icon strokes
and button/surface backgrounds for Nevermore themes, asserting contrast >= 3.0:1 (target 4.5:1+).
"""

import sys
if sys.version_info >= (3, 11):
    import tomllib as toml
else:
    import tomli as toml
from pathlib import Path

def srgb_to_linear(c: float) -> float:
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

def relative_luminance(hex_str: str) -> float:
    hex_str = hex_str.strip().lstrip('#')
    if len(hex_str) == 6:
        r, g, b = (int(hex_str[i:i+2], 16) / 255.0 for i in (0, 2, 4))
    elif len(hex_str) == 8:
        r, g, b = (int(hex_str[i:i+2], 16) / 255.0 for i in (0, 2, 4))
    else:
        raise ValueError(f"Invalid hex string: {hex_str}")
    
    r_lin = srgb_to_linear(r)
    g_lin = srgb_to_linear(g)
    b_lin = srgb_to_linear(b)
    return 0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin

def contrast_ratio(hex1: str, hex2: str) -> float:
    l1 = relative_luminance(hex1)
    l2 = relative_luminance(hex2)
    lighter = max(l1, l2)
    darker = min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)

def main():
    root = Path(__file__).resolve().parent.parent
    themes_dir = root / "crates" / "nevermore-core" / "resources" / "themes"

    target_themes = ["nevermore-dark.toml", "catppuccin-mocha.toml"]
    failures = []
    results = []

    print("================================================================================")
    print("NEVERMORE ICON CONTRAST VERIFICATION (WCAG 2.1)")
    print("================================================================================")

    for theme_file in target_themes:
        path = themes_dir / theme_file
        if not path.exists():
            print(f"[ERROR] Missing theme file: {path}")
            sys.exit(1)

        with open(path, "rb") as f:
            data = toml.load(f)

        theme_name = data.get("name", theme_file)
        bg_primary = data["bg_primary"]
        bg_secondary = data["bg_secondary"]
        bg_tertiary = data["bg_tertiary"]
        accent_color = data["accent_color"]
        text_secondary = data["text_secondary"]

        print(f"\n--- Theme: {theme_name} ({theme_file}) ---")
        print(f"  bg_primary:     {bg_primary}")
        print(f"  bg_secondary:   {bg_secondary}")
        print(f"  bg_tertiary:    {bg_tertiary}")
        print(f"  accent_color:   {accent_color}")
        print(f"  text_secondary: {text_secondary}")

        # Check 1: Inactive Nav Rail Icon (text_secondary on bg_secondary)
        cr_inactive_nav = contrast_ratio(text_secondary, bg_secondary)
        pass_inactive_nav = cr_inactive_nav >= 3.0
        status_in = "PASS" if pass_inactive_nav else "FAIL"
        print(f"  [1] Inactive Nav Icon ({text_secondary} on {bg_secondary}): {cr_inactive_nav:.2f}:1 [{status_in}]")
        results.append((theme_name, "Inactive Nav Icon", cr_inactive_nav, pass_inactive_nav))
        if not pass_inactive_nav:
            failures.append(f"{theme_name}: Inactive Nav Icon contrast {cr_inactive_nav:.2f}:1 < 3.0:1")

        # Check 2: Active Nav Tab Icon (accent_color on bg_tertiary)
        cr_active_nav = contrast_ratio(accent_color, bg_tertiary)
        pass_active_nav = cr_active_nav >= 3.0
        status_act = "PASS" if pass_active_nav else "FAIL"
        print(f"  [2] Active Nav Tab Icon ({accent_color} on {bg_tertiary}): {cr_active_nav:.2f}:1 [{status_act}]")
        results.append((theme_name, "Active Nav Tab Icon", cr_active_nav, pass_active_nav))
        if not pass_active_nav:
            failures.append(f"{theme_name}: Active Nav Tab Icon contrast {cr_active_nav:.2f}:1 < 3.0:1")

        # Check 3: Header Action Icons (pin / close) (text_secondary on bg_primary)
        cr_header_icon = contrast_ratio(text_secondary, bg_primary)
        pass_header = cr_header_icon >= 3.0
        status_hdr = "PASS" if pass_header else "FAIL"
        print(f"  [3] Header Action Icon ({text_secondary} on {bg_primary}): {cr_header_icon:.2f}:1 [{status_hdr}]")
        results.append((theme_name, "Header Action Icon", cr_header_icon, pass_header))
        if not pass_header:
            failures.append(f"{theme_name}: Header Action Icon contrast {cr_header_icon:.2f}:1 < 3.0:1")

        # Check 4: Pinned Button / Active DND (contrast text on accent_color)
        accent_fg = "#ffffff" if relative_luminance(accent_color) < 0.4 else "#11111b"
        cr_accent_btn = contrast_ratio(accent_fg, accent_color)
        pass_accent_btn = cr_accent_btn >= 3.0
        status_acc = "PASS" if pass_accent_btn else "FAIL"
        print(f"  [4] Active Toggle / Pin Icon ({accent_fg} on {accent_color}): {cr_accent_btn:.2f}:1 [{status_acc}]")
        results.append((theme_name, "Active Toggle / Pin Icon", cr_accent_btn, pass_accent_btn))
        if not pass_accent_btn:
            failures.append(f"{theme_name}: Active Toggle Icon contrast {cr_accent_btn:.2f}:1 < 3.0:1")

    print("\n================================================================================")
    print("SUMMARY")
    print("================================================================================")
    for theme_name, item, cr, passed in results:
        status = "PASSED (>= 3.0:1)" if passed else "FAILED"
        print(f"{theme_name:<20} | {item:<26} | {cr:5.2f}:1 | {status}")

    if failures:
        print(f"\n[FAIL] {len(failures)} checks failed minimum 3.0:1 contrast!")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    else:
        print("\n[SUCCESS] All icon contrast ratios meet or exceed 3.0:1 (target 4.5:1+)!")
        sys.exit(0)

if __name__ == "__main__":
    main()
