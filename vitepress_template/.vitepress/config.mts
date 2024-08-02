import { defineConfig } from "vitepress"
import { generateSidebar } from "vitepress-sidebar"

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Lua Reference",
    themeConfig: {
        // https://vitepress.dev/reference/default-theme-config
        nav: [
            { text: "Home", link: "/" },
        ],

        sidebar: generateSidebar({}),

        socialLinks: [
            { icon: "github", link: "https://github.com" }
        ],
        search: {
            provider: "local"
        },
        footer: {
            message: "Generated with <a href=\"https://github.com/Ottatop/lcat\">lcat</a>",
        },
    },
    lastUpdated: true,
    vite: {
        ssr: {
            noExternal: [
                "@nolebase/vitepress-plugin-highlight-targeted-heading",
            ],
        },
    },
})
