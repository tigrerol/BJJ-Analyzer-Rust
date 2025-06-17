#!/usr/bin/env python3
"""
Lightweight FastAPI server for remote Whisper GPU transcription
Designed for Docker deployment with minimal dependencies
"""

import os
import tempfile
import time
import logging
from typing import Optional, Dict, List, Any
from pathlib import Path

import uvicorn
from fastapi import FastAPI, File, UploadFile, Form, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import whisper

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Initialize FastAPI app
app = FastAPI(
    title="Remote Whisper GPU Server",
    description="High-performance Whisper transcription with GPU acceleration",
    version="1.0.0"
)

# Global model cache
_models: Dict[str, Any] = {}

class TranscriptionResponse(BaseModel):
    text: str
    language: str
    segments: List[Dict[str, Any]]
    processing_time: float
    model_used: str

class HealthResponse(BaseModel):
    status: str
    gpu_available: bool
    loaded_models: List[str]
    memory_usage: Optional[str] = None

@app.on_event("startup")
async def startup_event():
    """Initialize server and load default model"""
    logger.info("🚀 Starting Remote Whisper GPU Server")
    
    # Check GPU availability
    import torch
    gpu_available = torch.cuda.is_available()
    logger.info(f"🎮 GPU Available: {gpu_available}")
    
    if gpu_available:
        logger.info(f"🎮 GPU Count: {torch.cuda.device_count()}")
        for i in range(torch.cuda.device_count()):
            logger.info(f"🎮 GPU {i}: {torch.cuda.get_device_name(i)}")
    
    # Load default model
    try:
        default_model = os.getenv("DEFAULT_MODEL", "base")
        logger.info(f"📦 Loading default model: {default_model}")
        load_model(default_model)
        logger.info("✅ Default model loaded successfully")
    except Exception as e:
        logger.warning(f"⚠️  Failed to load default model: {e}")

def load_model(model_name: str) -> Any:
    """Load and cache a Whisper model"""
    if model_name in _models:
        return _models[model_name]
    
    logger.info(f"📦 Loading model: {model_name}")
    start_time = time.time()
    
    try:
        model = whisper.load_model(model_name)
        _models[model_name] = model
        load_time = time.time() - start_time
        logger.info(f"✅ Model '{model_name}' loaded in {load_time:.1f}s")
        return model
    except Exception as e:
        logger.error(f"❌ Failed to load model '{model_name}': {e}")
        raise HTTPException(status_code=500, detail=f"Failed to load model: {e}")

@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Health check endpoint"""
    import torch
    
    try:
        memory_info = None
        if torch.cuda.is_available():
            memory_allocated = torch.cuda.memory_allocated() / 1024**3  # GB
            memory_reserved = torch.cuda.memory_reserved() / 1024**3   # GB
            memory_info = f"{memory_allocated:.1f}GB allocated, {memory_reserved:.1f}GB reserved"
        
        return HealthResponse(
            status="healthy",
            gpu_available=torch.cuda.is_available(),
            loaded_models=list(_models.keys()),
            memory_usage=memory_info
        )
    except Exception as e:
        logger.error(f"Health check failed: {e}")
        return HealthResponse(
            status="unhealthy",
            gpu_available=False,
            loaded_models=[],
            memory_usage=f"Error: {e}"
        )

@app.post("/transcribe", response_model=TranscriptionResponse)
async def transcribe_audio(
    audio: UploadFile = File(..., description="Audio file to transcribe"),
    model: str = Form("base", description="Whisper model to use"),
    language: Optional[str] = Form(None, description="Language code (auto-detect if None)"),
    prompt: Optional[str] = Form(None, description="Initial prompt for better accuracy"),
    temperature: float = Form(0.0, description="Temperature for sampling (0.0 = deterministic)"),
    word_timestamps: bool = Form(True, description="Include word-level timestamps")
):
    """Transcribe audio file using Whisper"""
    start_time = time.time()
    temp_file = None
    
    try:
        # Validate file
        if not audio.filename:
            raise HTTPException(status_code=400, detail="No filename provided")
        
        file_size = 0
        content = await audio.read()
        file_size = len(content)
        
        logger.info(f"🎤 Transcription request: {audio.filename} ({file_size/1024/1024:.1f}MB)")
        logger.info(f"⚙️  Model: {model}, Language: {language or 'auto'}, Prompt: {bool(prompt)}")
        
        # Load model
        whisper_model = load_model(model)
        
        # Save uploaded file to temporary location
        with tempfile.NamedTemporaryFile(delete=False, suffix=Path(audio.filename).suffix) as tmp:
            tmp.write(content)
            temp_file = tmp.name
        
        logger.info(f"💾 Saved to temp file: {temp_file}")
        
        # Prepare transcription options
        transcribe_options = {
            "temperature": temperature,
            "word_timestamps": word_timestamps,
            "verbose": False  # Disable verbose logging to avoid cluttering
        }
        
        if language:
            transcribe_options["language"] = language
        
        if prompt and prompt.strip():
            transcribe_options["initial_prompt"] = prompt.strip()
        
        # Transcribe
        logger.info("🚀 Starting transcription...")
        transcription_start = time.time()
        
        result = whisper_model.transcribe(temp_file, **transcribe_options)
        
        transcription_time = time.time() - transcription_start
        logger.info(f"✅ Transcription completed in {transcription_time:.1f}s")
        
        # Format response
        segments = []
        for i, segment in enumerate(result.get("segments", [])):
            segment_data = {
                "id": i,
                "start": segment.get("start", 0.0),
                "end": segment.get("end", 0.0),
                "text": segment.get("text", "").strip()
            }
            
            # Add optional fields if available
            if "avg_logprob" in segment:
                segment_data["avg_logprob"] = segment["avg_logprob"]
            if "no_speech_prob" in segment:
                segment_data["no_speech_prob"] = segment["no_speech_prob"]
            if word_timestamps and "words" in segment:
                segment_data["words"] = segment["words"]
            
            segments.append(segment_data)
        
        processing_time = time.time() - start_time
        
        response = TranscriptionResponse(
            text=result["text"].strip(),
            language=result.get("language", "unknown"),
            segments=segments,
            processing_time=processing_time,
            model_used=model
        )
        
        logger.info(f"🎉 Response ready: {len(response.text)} chars, {len(segments)} segments, {processing_time:.1f}s total")
        
        return response
        
    except Exception as e:
        logger.error(f"❌ Transcription failed: {e}")
        raise HTTPException(status_code=500, detail=f"Transcription failed: {str(e)}")
    
    finally:
        # Cleanup temporary file
        if temp_file and os.path.exists(temp_file):
            try:
                os.unlink(temp_file)
                logger.debug(f"🗑️  Cleaned up temp file: {temp_file}")
            except Exception as e:
                logger.warning(f"⚠️  Failed to cleanup temp file: {e}")

@app.get("/models")
async def list_models():
    """List available Whisper models"""
    available_models = [
        "tiny", "tiny.en",
        "base", "base.en", 
        "small", "small.en",
        "medium", "medium.en",
        "large", "large-v1", "large-v2", "large-v3"
    ]
    
    return {
        "available_models": available_models,
        "loaded_models": list(_models.keys()),
        "recommended": "base" if not _models else list(_models.keys())[0]
    }

@app.post("/models/{model_name}/load")
async def preload_model(model_name: str):
    """Preload a specific model"""
    try:
        load_model(model_name)
        return {"status": "success", "message": f"Model '{model_name}' loaded successfully"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.delete("/models/{model_name}")
async def unload_model(model_name: str):
    """Unload a specific model to free memory"""
    if model_name in _models:
        del _models[model_name]
        
        # Force garbage collection
        import gc
        gc.collect()
        
        # Clear CUDA cache if available
        try:
            import torch
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except:
            pass
            
        logger.info(f"🗑️  Unloaded model: {model_name}")
        return {"status": "success", "message": f"Model '{model_name}' unloaded"}
    else:
        raise HTTPException(status_code=404, detail=f"Model '{model_name}' not loaded")

if __name__ == "__main__":
    # Configuration
    host = os.getenv("HOST", "0.0.0.0")
    port = int(os.getenv("PORT", "8080"))
    workers = int(os.getenv("WORKERS", "1"))
    
    logger.info(f"🌟 Starting server on {host}:{port} with {workers} workers")
    
    uvicorn.run(
        "main:app",
        host=host,
        port=port,
        workers=workers,
        log_level="info",
        access_log=True
    )