# cosmic-bundle-validator

Checks a COSMIC theme bundle with the same importer COSMIC Settings uses.

```
cosmic-bundle-validator validate <bundle.zip>
cosmic-bundle-validator build <id.xml> [--out <dir>]
cosmic-bundle-validator catalog <bundle-dir> <out-dir>
```

`validate` imports the zip into a throwaway home, exactly as `cosmic-settings` would, and fails with Settings' own error if the import would.

`build` is for `cosmic-constellations`. It reads a pin file, checks it, fetches the pinned commit, zips the bundle with `git archive`, then validates it and writes
`<dir>/ID.zip`. A pin file looks like this:

```xml
<bundle id="com.example.theme">
  <!-- path is optional, for repos that hold several bundles -->
  <source type="git" url="https://github.com/you/your-theme" commit="<full 40-char sha>" path="dark"/>
  <developer id="io.github.you">
    <name>Your Name</name>
  </developer>
  <!-- an SPDX expression, or Proprietary -->
  <project_license>GPL-3.0-or-later</project_license>
  <!-- optional -->
  <metadata_license>CC0-1.0</metadata_license>
  <!-- optional, each image must be an https URL -->
  <screenshots>
    <screenshot type="default">
      <image>https://example.com/screenshot.png</image>
      <caption>The desktop with this theme</caption>
    </screenshot>
  </screenshots>
</bundle>
```
