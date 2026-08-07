# SidebarFlipIcon Component

This is a Vue component wrapping the sidebar-flip SVG icon, ready to use directly in a Nuxt project.

## Usage

### 1. Import the component

```vue
<script setup lang="ts">
import SidebarFlipIcon from "~/icons/SidebarFlipIcon.vue";
</script>
```

### 2. Use it in the template

```vue
<template>
  <!-- Basic usage -->
  <SidebarFlipIcon />

  <!-- Custom size -->
  <SidebarFlipIcon size="32" />

  <!-- Custom style -->
  <SidebarFlipIcon size="24" class="text-blue-500" />

  <!-- Different size examples -->
  <SidebarFlipIcon size="16" class="text-gray-500" />
  <SidebarFlipIcon size="24" class="text-blue-500" />
  <SidebarFlipIcon size="32" class="text-green-500" />
  <SidebarFlipIcon size="48" class="text-purple-500" />
</template>
```

## Props

| Prop  | Type             | Default | Description         |
| ----- | ---------------- | ------- | -------------------- |
| size  | string \| number | '24'    | Icon size            |
| class | string           | ''      | Custom CSS class name |

## Features

- ✅ Supports custom sizing
- ✅ Supports custom styling (via the class prop)
- ✅ Filled with `currentColor`, inheriting the parent element's text color
- ✅ TypeScript support
- ✅ Responsive design

## Test page

Visit the `/test-icon` page to see various usage examples of the icon.
