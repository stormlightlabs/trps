// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import linksValidator from "starlight-links-validator";

// `site` stays unset until the documentation has a hostname. starlight-llms-txt
// refuses to run without one, so it joins the config alongside the domain.
export default defineConfig({
  integrations: [
    starlight({
      plugins: [linksValidator()],
      title: "tropius",
      description: "A CLI that detects AI tropes in prose.",
      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/stormlightlabs/trps" },
      ],
      sidebar: [{ label: "Documentation", items: [{ autogenerate: { directory: "." } }] }],
    }),
  ],
});
