#!/usr/bin/env zx

const { release } = argv;

if (release !== "major" && release !== "minor" && release !== "patch") {
  throw new Error(
    `${value} is not valid release. Must be 'major' or 'minor' or 'patch'.`
  );
}

await $`npm version ${release}`;

await $`git push origin HEAD`;

await $`git push origin --tags`;

const { version } = JSON.parse(
  await fs.readFile(path.join(__dirname, "..", "package.json"))
);

console.log("Done!");

console.log(
  `Open https://github.com/sosukesuzuki/swc-plugin-valtio/releases/new?tag=v${version} and create new release.`
);
