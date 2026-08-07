export default defineAppConfig({
  app: {
    name: "JumpServer Client",
    author: "JumpServer",
    version: "4.0.0",
    repo: "https://github.com/matheus-marques-ft/js-client"
  },
  componentsConfig: {
    header: {
      // Colors are now managed via CSS variables, defined in main.css
      // Kept here for any other possible configuration
    },
    pages: {
      scrollBarLightThumbColor: "#D0D1D2",
      scrollBarDarkThumbColor: "#4A4A4A",
      scrollBarLightHoverColor: "#B8B9BA",
      scrollBarDarkHoverColor: "#6B6B6B",
      mainCardLightBackgroundColor: "#FAFAFA",
      mainCardDarkBackgroundColor: "#2C2C2C"
    },
    urlRegExp:
      /^(?:https?:\/\/(?:localhost|\d{1,3}(?:\.\d{1,3}){3}|\[[0-9a-fA-F:]+\]|(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,})(?::\d{1,5})?(?:[/?#]\S*)?|\d{1,3}(?:\.\d{1,3}){3}|\[[0-9a-fA-F:]+\])$/
  },
  ui: {
    fonts: false,
    colors: {
      primary: "primary",
      neutral: "zinc"
    },
    container: {
      base: "mx-0 w-full"
    },
    button: {
      slots: {
        base: "cursor-pointer"
      },
      variants: {
        ghost: {
          neutral: {
            base: "bg-transparent hover:bg-gray-50"
          }
        }
      }
    },
    toast: {
      slots: {
        title: "select-text",
        description: "text-sm text-muted whitespace-pre-wrap break-all select-text"
      }
    },
    formField: {
      slots: {
        root: "w-full"
      }
    },
    input: {
      slots: {
        root: "w-full"
      }
    },
    textarea: {
      slots: {
        root: "w-full",
        base: "resize-none"
      }
    },
    accordion: {
      slots: {
        trigger: "cursor-pointer",
        item: "md:py-2"
      }
    },
    dropdownMenu: {
      slots: {
        content: "w-(--reka-dropdown-menu-trigger-width) p-1",
        item: "mx-0.5 px-3 py-2 rounded-md transition-colors duration-150"
      }
    },
    navigationMenu: {
      slots: {
        link: "cursor-pointer"
      },
      variants: {
        disabled: {
          true: {
            link: "cursor-text"
          }
        }
      }
    }
  }
});
