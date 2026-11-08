#!/usr/bin/env python3
"""Motore immagini neurale opzionale per CAA Writer.

Non fa parte del bundle Python dell'app: viene eseguito con un interprete
Python esterno in cui sono installati torch/diffusers (GPU Apple via MPS) e,
se presenti, MLX e il runtime MLX Stable Diffusion.

Comandi:
    image_engine.py --status
    image_engine.py --download --model segmind/tiny-sd --cache DIR
    image_engine.py --generate --prompt "..." --out file.png \
        --model segmind/tiny-sd --cache DIR --backend auto \
        --size 512 --steps 16 --guidance 7.0

L'output di --status, --download e --generate è un oggetto JSON su stdout.
Nessun testo del documento viene inviato: solo il prompt scritto dall'utente.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path


def probe() -> dict:
    state = {
        "python": sys.version.split()[0],
        "mlx": False,
        "mlx_diffusion": False,
        "torch": False,
        "mps": False,
        "diffusers": False,
    }
    try:
        import mlx.core  # noqa: F401

        state["mlx"] = True
    except Exception:
        pass
    try:
        import stable_diffusion  # noqa: F401  (mlx-examples)

        state["mlx_diffusion"] = True
    except Exception:
        pass
    try:
        import torch

        state["torch"] = True
        try:
            state["mps"] = bool(torch.backends.mps.is_available())
        except Exception:
            state["mps"] = False
    except Exception:
        pass
    try:
        import diffusers  # noqa: F401

        state["diffusers"] = True
    except Exception:
        pass
    return state


def load_pipeline(model: str, cache: str):
    import torch
    from diffusers import AutoPipelineForText2Image

    device = "mps" if torch.backends.mps.is_available() else "cpu"
    dtype = torch.float16 if device == "mps" else torch.float32
    pipeline = AutoPipelineForText2Image.from_pretrained(
        model,
        torch_dtype=dtype,
        cache_dir=cache or None,
        safety_checker=None,
        requires_safety_checker=False,
    )
    pipeline = pipeline.to(device)
    # 8 GB di RAM non bastano per 512×512 senza slicing: si riduce il picco
    # di memoria dell'attenzione e del VAE invece di andare in OOM.
    try:
        pipeline.enable_attention_slicing("max")
    except Exception:
        pass
    try:
        pipeline.enable_vae_slicing()
    except Exception:
        pass
    try:
        pipeline.enable_vae_tiling()
    except Exception:
        pass
    pipeline.set_progress_bar_config(disable=True)
    return pipeline, device


def download(args) -> int:
    try:
        load_pipeline(args.model, args.cache)
    except Exception as error:  # pragma: no cover - dipende dalla rete
        print(json.dumps({"ok": False, "error": str(error)}))
        return 1
    print(json.dumps({"ok": True, "model": args.model}))
    return 0


def generate(args) -> int:
    started = time.time()
    state = probe()
    backend = args.backend
    if backend == "mlx" and not (state["mlx"] and state["mlx_diffusion"]):
        # MLX presente ma senza il runtime Stable Diffusion: si usa la GPU.
        backend = "gpu"
    if backend == "auto":
        backend = "gpu"
    if backend == "mlx" and state["mlx"] and state["mlx_diffusion"]:
        try:
            from stable_diffusion import generate_image  # type: ignore

            image = generate_image(
                args.prompt,
                model=args.model,
                steps=args.steps,
                seed=args.seed,
            )
            image.save(args.out)
            print(
                json.dumps(
                    {
                        "ok": True,
                        "backend": "mlx",
                        "model": args.model,
                        "width": image.width,
                        "height": image.height,
                        "elapsed_s": round(time.time() - started, 1),
                    }
                )
            )
            return 0
        except Exception:
            backend = "gpu"

    try:
        pipeline, device = load_pipeline(args.model, args.cache)
        # I modelli "turbo"/"lightning" sono distillati per pochi passi e
        # guidance 0: usare i valori di SD1.5 li rende inutilizzabili.
        steps = args.steps
        guidance = args.guidance
        if "turbo" in args.model.lower() or "lightning" in args.model.lower():
            steps = min(steps, 4)
            guidance = 0.0
        kwargs = {
            "prompt": args.prompt,
            "num_inference_steps": steps,
            "guidance_scale": guidance,
            "width": args.size,
            "height": args.size,
        }
        if args.negative:
            kwargs["negative_prompt"] = args.negative
        image = pipeline(**kwargs).images[0]
        out = Path(args.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        image.save(out, format="PNG")
        print(
            json.dumps(
                {
                    "ok": True,
                    "backend": "mps" if device == "mps" else "cpu",
                    "model": args.model,
                    "width": image.width,
                    "height": image.height,
                    "elapsed_s": round(time.time() - started, 1),
                }
            )
        )
        return 0
    except Exception as error:  # pragma: no cover - dipende da runtime/rete
        print(json.dumps({"ok": False, "error": str(error)}))
        return 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--download", action="store_true")
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--prompt", default="")
    parser.add_argument("--negative", default="")
    parser.add_argument("--out", default="")
    parser.add_argument("--model", default="stabilityai/sd-turbo")
    parser.add_argument("--cache", default="")
    parser.add_argument("--backend", default="auto")
    parser.add_argument("--size", type=int, default=512)
    parser.add_argument("--steps", type=int, default=16)
    parser.add_argument("--guidance", type=float, default=7.0)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    if args.status:
        print(json.dumps(probe()))
        return 0
    if args.download:
        return download(args)
    if args.generate:
        if not args.prompt.strip():
            print(json.dumps({"ok": False, "error": "prompt vuoto"}))
            return 2
        if not args.out:
            print(json.dumps({"ok": False, "error": "serve --out"}))
            return 2
        return generate(args)
    parser.print_help()
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
