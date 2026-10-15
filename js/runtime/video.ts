// `nsh/video`: video playback.

import { hostComponent, type YNode } from "./_core";
import type { VideoHostProps, ViewProps } from "./types";

export interface VideoProps extends Omit<ViewProps, "children" | "onLoad" | "onError">, VideoHostProps {
  source: string | { uri: string };
  onLoadStart?: () => void;
  onLoad?: (e: { duration: number; width: number; height: number }) => void;
  onProgress?: (e: { currentTime: number; duration: number }) => void;
  onEnd?: () => void;
  onBuffer?: (e: { isBuffering: boolean }) => void;
  onError?: (e: { message: string }) => void;
}

/** A video player (AVPlayer / MediaPlayer); its `ref` has `seek(seconds)`. */
export const Video: (props: VideoProps) => YNode = hostComponent<VideoProps>("video");
