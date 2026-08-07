import { ref } from "vue";

interface HSL {
  h: number
  s: number
  l: number
}

interface RGB {
  r: number
  g: number
  b: number
}

const mainThemeColorMap = new Map(
  Object.entries({
    darkGary: "#303237"
  })
);

const currentMainColoc = ref("#303237");

export const useColor = () => {
  const setCurrentMainColor = (color: string) => {
    const themeColor = mainThemeColorMap.get(color);

    if (themeColor) {
      currentMainColoc.value = themeColor;
    } else {
      currentMainColoc.value = "#303237";
    }
  };

  /**
   * Normalize a 3/6/8-digit hex value to #RRGGBB (ignoring alpha)
   */
  const normalizeHex = (hex: string): string | null => {
    if (!hex) return null;
    let v = hex.trim().replace(/^#/, "");
    if (!v) return null;
    if (v.length === 3) {
      v = v
        .split("")
        .map((c) => c + c)
        .join("");
    } else if (v.length === 8) {
      v = v.substring(0, 6); // discard alpha
    } else if (v.length !== 6) {
      return null;
    }
    if (!/^[0-9a-f]{6}$/i.test(v)) return null;
    return `#${v.toLowerCase()}`;
  };

  /**
   * Convert #RGB/#RRGGBB to RGB numeric values
   */
  const hexToRgb = (hex: string): RGB => {
    const n = normalizeHex(hex);
    const safe = n || "#000000";
    const v = safe.replace(/^#/, "");
    const r = Number.parseInt(v.substring(0, 2), 16);
    const g = Number.parseInt(v.substring(2, 4), 16);
    const b = Number.parseInt(v.substring(4, 6), 16);
    return { r, g, b };
  };

  /**
   * Convert RGB numeric values to #RRGGBB
   */
  const rgbToHex = (r: number, g: number, b: number): string => {
    const to2 = (n: number) =>
      Math.max(0, Math.min(255, Math.round(n)))
        .toString(16)
        .padStart(2, "0");
    return `#${to2(r)}${to2(g)}${to2(b)}`;
  };

  /**
   * Parse rgb()/rgba()/hex, returning #RRGGBB (ignoring alpha). Returns the current theme's primary color if parsing fails.
   */
  const toHex = (input: string, fallback?: string): string => {
    // Try hex first
    const n = normalizeHex(input);
    if (n) return n;

    // Then try rgb/rgba
    const m = (input || "").trim().match(/rgba?\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})/i);
    if (m) {
      const r = Math.min(255, Number.parseInt(m[1], 10));
      const g = Math.min(255, Number.parseInt(m[2], 10));
      const b = Math.min(255, Number.parseInt(m[3], 10));
      return rgbToHex(r, g, b);
    }

    // Fallback: use the passed-in fallback, the current primary color, or the default value
    return normalizeHex(fallback || currentMainColoc.value || "#303237") || "#303237";
  };

  /**
   * Convert a hex color to an HSL color
   * @param hex hex color
   * @returns HSL color
   */
  const hexToHSL = (hex: string): HSL => {
    let hexValue = hex.replace(/^#/, "");

    if (hexValue.length === 3) {
      hexValue = hexValue
        .split("")
        .map((char) => char + char)
        .join("");
    }

    // Parse the RGB values
    const r = Number.parseInt(hexValue.substring(0, 2), 16) / 255;
    const g = Number.parseInt(hexValue.substring(2, 4), 16) / 255;
    const b = Number.parseInt(hexValue.substring(4, 6), 16) / 255;

    // Compute the HSL values
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = 0;
    let s = 0;
    const l = (max + min) / 2;

    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

      switch (max) {
        case r:
          h = (g - b) / d + (g < b ? 6 : 0);
          break;
        case g:
          h = (b - r) / d + 2;
          break;
        case b:
          h = (r - g) / d + 4;
          break;
      }

      h /= 6;
    }

    // Convert to the standard HSL format
    return {
      h: Math.round(h * 360),
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    };
  };

  /**
   * Convert an HSL color to a hex color
   * @param h hue
   * @param s saturation
   * @param l lightness
   * @returns hex color
   */
  const hslToHex = (h: number, s: number, l: number) => {
    h /= 360;
    s /= 100;
    l /= 100;

    let r, g, b;

    if (s === 0) {
      // Grayscale if saturation is 0
      r = g = b = l;
    } else {
      const hue2rgb = (p: number, q: number, t: number): number => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1 / 6) return p + (q - p) * 6 * t;
        if (t < 1 / 2) return q;
        if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
        return p;
      };

      const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
      const p = 2 * l - q;

      r = hue2rgb(p, q, h + 1 / 3);
      g = hue2rgb(p, q, h);
      b = hue2rgb(p, q, h - 1 / 3);
    }

    // Convert to hex
    const toHex = (x: number): string => {
      const hex = Math.round(x * 255).toString(16);
      return hex.length === 1 ? `0${hex}` : hex;
    };

    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  };

  /**
   * Convert a color to rgba format
   * @param alphaValue opacity value
   * @param color color
   * @returns rgba-format color
   */
  const alpha = (alphaValue: number, color?: string) => {
    // Use the current theme color if no color is provided
    const actualColor = color || currentMainColoc.value;
    // Make sure the opacity value is between 0 and 1
    const alpha = Math.max(0, Math.min(1, alphaValue));

    // Strip the # and handle the shorthand form
    let hex = (normalizeHex(actualColor) || currentMainColoc.value).replace(/^#/, "");

    if (hex.length === 3) {
      hex = hex
        .split("")
        .map((char) => char + char)
        .join("");
    }

    // Parse the RGB values
    const r = Number.parseInt(hex.substring(0, 2), 16);
    const g = Number.parseInt(hex.substring(2, 4), 16);
    const b = Number.parseInt(hex.substring(4, 6), 16);

    // Return rgba format
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  };

  /**
   * Lighten a color
   * @param amount
   * @param color
   * @param alphaValue
   */
  const lighten = (amount: number, color?: string, alphaValue?: number) => {
    const actualColor = color || currentMainColoc.value;
    const hsl = hexToHSL(actualColor);
    const hexColor = hslToHex(hsl.h, hsl.s, Math.min(100, hsl.l + amount));

    if (alphaValue !== undefined) {
      return alpha(alphaValue, hexColor);
    }

    return hexColor;
  };

  /**
   * Darken a color
   * @param amount
   * @param color
   * @param alphaValue
   */
  const darken = (amount: number, color?: string, alphaValue?: number) => {
    const actualColor = color || currentMainColoc.value;
    const hsl = hexToHSL(actualColor);
    const hexColor = hslToHex(hsl.h, hsl.s, Math.max(0, hsl.l - amount));

    // Apply opacity if the opacity parameter was provided
    if (alphaValue !== undefined) {
      return alpha(alphaValue, hexColor);
    }

    return hexColor;
  };

  const applyPrimaryColor = (color: string, fallback: string = "#1ab394") => {
    const root = document.documentElement;

    const hex = toHex(color, fallback);
    root.style.setProperty("--ui-color-primary-500", hex);
    root.style.setProperty("--el-color-primary", hex);

    try {
      const c400 = lighten(8, hex);
      const c600 = darken(8, hex);
      const c700 = darken(14, hex);
      const c300 = lighten(16, hex);
      root.style.setProperty("--ui-color-primary-300", c300);
      root.style.setProperty("--ui-color-primary-400", c400);
      root.style.setProperty("--ui-color-primary-600", c600);
      root.style.setProperty("--ui-color-primary-700", c700);

      root.style.setProperty("--el-color-primary-light-3", lighten(30, hex));
      root.style.setProperty("--el-color-primary-light-5", lighten(50, hex));
      root.style.setProperty("--el-color-primary-light-7", lighten(70, hex));
      root.style.setProperty("--el-color-primary-light-8", lighten(80, hex));
      root.style.setProperty("--el-color-primary-light-9", lighten(90, hex));
      root.style.setProperty("--el-color-primary-dark-2", darken(20, hex));
    } catch {}

    return hex;
  };

  return {
    darken,
    lighten,
    alpha,
    toHex,
    hexToRgb,
    rgbToHex,
    applyPrimaryColor,
    setCurrentMainColor
  };
};
