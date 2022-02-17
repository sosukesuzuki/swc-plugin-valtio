# swc-plugin-valtio

[Valtio `useProxy` transformer](https://github.com/pmndrs/valtio#useproxy-macro) for SWC.

## Installation

```
npm install --save-dev swc-plugin-valtio
```

## Usage

```json
{
  "jsc": {
    "experimental": {
      "plugins": [["swc-plugin-valtio", {}]]
    }
  }
}
```

## Example

```jsx
import { useProxy } from 'valtio/macro'

const Component = () => {
  useProxy(state);
  return (
    <div>
      {state.count}
      <button onClick={() => ++state.count}>
        +1
      </button>
    </div>
  );
}

// The code above becomes the code below.

import { useSnapshot } from 'valtio';

const Component = () => {
  const valtio_macro_snap_state = useSnapshot(state);
  return (
    <div>
      {valtio_macro_snap_state.count}
      <button onClick={() => ++state.count}>
        +1
      </button>
    </div>
  );
}

```

## Tests

For MacOS

```
cargo test_darwin
```

For Linux

```
cargo test_linux
```

## Release

```zsh
npm run release -- --patch
```

```zsh
npm run release -- --minor
```

```zsh
npm run release -- --major
```
