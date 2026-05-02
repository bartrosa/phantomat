import { expectError, expectType } from "tsd";
import {
  SceneBuilder,
  ScatterLayer,
  sceneBuilder,
} from "../src/index.js";

expectType<SceneBuilder>(
  sceneBuilder().scatter({
    positions: new Float32Array(0),
    colors: new Float32Array(0),
    sizes: new Float32Array(0),
  }),
);

expectError(new ScatterLayer({}));
