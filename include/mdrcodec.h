#ifndef MDRCODEC_H
#define MDRCODEC_H "0.1.0"

typedef struct mdrcodec_streaming_codec mdrcodec_streaming_codec;

*mdrcodec_streaming_codec mdrcoder_basic_encoder_new(int fd);
void mdrcoder_basic_encoder_drop(*mdrcodec_streaming_codec self);

#endif // MDRCODEC_H
