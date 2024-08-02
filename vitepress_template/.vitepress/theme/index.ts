import DefaultTheme from "vitepress/theme"
import "./custom.css"
import type { Theme as ThemeConfig } from "vitepress"
import { h } from "vue"

import {
    NolebaseHighlightTargetedHeading,
} from '@nolebase/vitepress-plugin-highlight-targeted-heading/client'

import '@nolebase/vitepress-plugin-highlight-targeted-heading/client/style.css'

export const Theme: ThemeConfig = {
    extends: DefaultTheme,
    Layout: () => {
        return h(DefaultTheme.Layout, null, {
            "layout-top": () => [
                h(NolebaseHighlightTargetedHeading)
            ]
        })
    }
}

export default Theme
