FROM node:22-alpine AS builder

RUN apk add --no-cache git
WORKDIR /app
COPY package.json pnpm-lock.yaml* .npmrc* ./
RUN npm install -g pnpm && pnpm install --config.block-exotic-subdeps=false

COPY . .
RUN pnpm build

FROM node:22-alpine AS runner
RUN apk add --no-cache git
WORKDIR /app
ENV NODE_ENV=production

COPY package.json pnpm-lock.yaml* .npmrc* ./
RUN npm install -g pnpm && pnpm install --prod --config.block-exotic-subdeps=false

COPY --from=builder /app/dist ./dist

CMD ["node", "dist/index.js"]
