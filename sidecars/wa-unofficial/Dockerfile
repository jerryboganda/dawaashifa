FROM node:22-alpine AS builder

RUN apk add --no-cache git
WORKDIR /app
COPY package.json pnpm-lock.yaml* ./
RUN npm install -g pnpm && pnpm install

COPY . .
RUN pnpm build

FROM node:22-alpine AS runner
RUN apk add --no-cache git
WORKDIR /app
ENV NODE_ENV=production

COPY package.json pnpm-lock.yaml* ./
RUN npm install -g pnpm && pnpm install --prod

COPY --from=builder /app/dist ./dist

CMD ["node", "dist/index.js"]
