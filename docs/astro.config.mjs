// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import linksValidator from "starlight-links-validator";
import llmsTxt from "starlight-llms-txt";

export default defineConfig({
  site: "https://trps.stormlightlabs.org",
  integrations: [
    starlight({
      plugins: [linksValidator(), llmsTxt()],
      title: "tropius",
      description: "A CLI that detects AI tropes in prose.",
      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/stormlightlabs/trps" },
      ],
      customCss: [
        "@fontsource-variable/google-sans",
        "@fontsource-variable/inter",
        "@fontsource-variable/google-sans-code",
        "./src/styles/theme.css",
      ],
      sidebar: [{ label: "Documentation", items: [{ autogenerate: { directory: "." } }] }],
    }),
  ],
});
