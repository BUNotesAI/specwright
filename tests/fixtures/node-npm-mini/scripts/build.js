const args = process.argv.slice(2);

if (args.length !== 0) {
  console.error(`unexpected build argv: ${JSON.stringify(args)}`);
  process.exit(1);
}

console.log("NODE_BUILD_OK");
