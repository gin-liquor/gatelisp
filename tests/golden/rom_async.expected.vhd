library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_rom_async is
  port (
    gl_p0_address : in unsigned(2 downto 0);
    gl_p1_value : out unsigned(7 downto 0)
  );
end entity gl_m0_rom_async;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_rom_async is
  type gl_rom_table_t is array (
    0 to 7
) of unsigned(7 downto 0);
  constant gl_rom_table : gl_rom_table_t := (
    0 => "00000001",
    2 => "00000100",
    7 => "11111111",
    others => "00000000"
  );
  signal gl_s1_value : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_value <= gl_rom_table(to_integer(gl_p0_address));
  end process gl_comb_0;
  gl_p1_value <= gl_s1_value;
end architecture rtl;
